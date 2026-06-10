use std::io;
use std::time::Duration;

use chrono::Utc;
use codex_client::build_reqwest_client_with_custom_ca;
use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::Validation;
use jsonwebtoken::decode;
use jsonwebtoken::decode_header;
use jsonwebtoken::jwk::JwkSet;
use serde::Deserialize;
use serde_json::Value;

use crate::server::OidcDiscovery;

const OIDC_HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const OIDC_CLOCK_SKEW_SECS: u64 = 60;
const OIDC_MAX_FUTURE_IAT_SECS: i64 = 60;
const ALLOWED_ID_TOKEN_ALGS: &[Algorithm] = &[Algorithm::RS256, Algorithm::ES256];

#[derive(Debug, Clone)]
pub(crate) struct VerifiedIdToken {
    pub(crate) raw: String,
    pub(crate) claims: OidcIdTokenClaims,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct OidcIdTokenClaims {
    pub(crate) iss: String,
    pub(crate) sub: String,
    pub(crate) aud: Audience,
    pub(crate) exp: usize,
    #[serde(default)]
    pub(crate) nbf: Option<usize>,
    #[serde(default)]
    pub(crate) iat: Option<usize>,
    #[serde(default)]
    pub(crate) nonce: Option<String>,
    #[serde(default)]
    pub(crate) azp: Option<String>,
    #[serde(flatten)]
    pub(crate) extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn values(&self) -> Vec<&str> {
        match self {
            Audience::One(value) => vec![value.as_str()],
            Audience::Many(values) => values.iter().map(String::as_str).collect(),
        }
    }

    fn contains(&self, expected: &str) -> bool {
        self.values().iter().any(|value| *value == expected)
    }

    fn len(&self) -> usize {
        self.values().len()
    }
}

pub(crate) fn validate_discovery(
    discovery: &OidcDiscovery,
    configured_issuer: &str,
    require_device_endpoint: bool,
) -> io::Result<()> {
    let expected_issuer = configured_issuer.trim_end_matches('/');
    let actual_issuer = discovery.issuer.trim_end_matches('/');
    if actual_issuer != expected_issuer {
        return Err(io::Error::other(format!(
            "OIDC discovery issuer mismatch: expected {expected_issuer}, got {actual_issuer}"
        )));
    }

    require_trusted_url("issuer", actual_issuer)?;
    require_trusted_url("authorization_endpoint", &discovery.authorization_endpoint)?;
    require_trusted_url("token_endpoint", &discovery.token_endpoint)?;
    require_trusted_url("jwks_uri", &discovery.jwks_uri)?;
    if require_device_endpoint {
        let endpoint = discovery.device_authorization_endpoint.as_deref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "device code login is not supported by this server (no device_authorization_endpoint in discovery)",
            )
        })?;
        require_trusted_url("device_authorization_endpoint", endpoint)?;
    }

    if let Some(methods) = discovery.code_challenge_methods_supported.as_ref()
        && !methods.iter().any(|method| method == "S256")
    {
        return Err(io::Error::other(
            "OIDC discovery does not advertise PKCE S256 support",
        ));
    }

    if let Some(algs) = discovery.id_token_signing_alg_values_supported.as_ref()
        && !algs
            .iter()
            .any(|alg| parse_allowed_algorithm(alg).is_some())
    {
        return Err(io::Error::other(
            "OIDC discovery does not advertise a supported ID token signing algorithm",
        ));
    }

    Ok(())
}

pub(crate) async fn verify_id_token(
    raw_id_token: &str,
    discovery: &OidcDiscovery,
    client_id: &str,
    expected_nonce: Option<&str>,
) -> io::Result<VerifiedIdToken> {
    if raw_id_token.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "OIDC token response did not include an id_token",
        ));
    }

    let header = decode_header(raw_id_token).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to decode ID token header: {err}"),
        )
    })?;
    let alg = header.alg;
    if !ALLOWED_ID_TOKEN_ALGS.contains(&alg) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported ID token signing algorithm: {alg:?}"),
        ));
    }
    if let Some(discovery_algs) = discovery.id_token_signing_alg_values_supported.as_ref()
        && !discovery_algs
            .iter()
            .filter_map(|alg| parse_allowed_algorithm(alg))
            .any(|allowed| allowed == alg)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("ID token algorithm {alg:?} is not advertised by discovery"),
        ));
    }

    let kid = header.kid.as_deref().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "ID token header does not include a kid",
        )
    })?;
    let jwks = fetch_jwks(&discovery.jwks_uri).await?;
    let jwk = jwks.find(kid).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("ID token kid {kid} is not present in JWKS"),
        )
    })?;
    let decoding_key = DecodingKey::from_jwk(jwk).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to build ID token decoding key: {err}"),
        )
    })?;

    let mut validation = Validation::new(alg);
    validation.leeway = OIDC_CLOCK_SKEW_SECS;
    validation.validate_nbf = true;
    validation.set_audience(&[client_id]);
    validation.set_issuer(&[discovery.issuer.as_str()]);
    validation.required_spec_claims.insert("iss".to_string());
    validation.required_spec_claims.insert("aud".to_string());
    validation.required_spec_claims.insert("exp".to_string());
    validation.required_spec_claims.insert("sub".to_string());

    let token =
        decode::<OidcIdTokenClaims>(raw_id_token, &decoding_key, &validation).map_err(|err| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("failed to verify ID token: {err}"),
            )
        })?;
    let claims = token.claims;
    validate_claims(&claims, client_id, expected_nonce)?;

    Ok(VerifiedIdToken {
        raw: raw_id_token.to_string(),
        claims,
    })
}

fn validate_claims(
    claims: &OidcIdTokenClaims,
    client_id: &str,
    expected_nonce: Option<&str>,
) -> io::Result<()> {
    if claims.sub.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ID token subject is empty",
        ));
    }
    if !claims.aud.contains(client_id) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ID token audience does not include the Sakrylle CLI client id",
        ));
    }
    if claims.aud.len() > 1 && claims.azp.as_deref() != Some(client_id) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ID token azp must equal the Sakrylle CLI client id for multi-audience tokens",
        ));
    }
    if let Some(expected_nonce) = expected_nonce
        && claims.nonce.as_deref() != Some(expected_nonce)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ID token nonce did not match the login attempt",
        ));
    }
    if let Some(iat) = claims.iat {
        let now = Utc::now().timestamp();
        if iat as i64 > now + OIDC_MAX_FUTURE_IAT_SECS {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "ID token issued-at time is in the future",
            ));
        }
    }
    Ok(())
}

async fn fetch_jwks(jwks_uri: &str) -> io::Result<JwkSet> {
    require_trusted_url("jwks_uri", jwks_uri)?;
    let client = build_reqwest_client_with_custom_ca(reqwest::Client::builder())?;
    let resp = client
        .get(jwks_uri)
        .timeout(OIDC_HTTP_TIMEOUT)
        .send()
        .await
        .map_err(io::Error::other)?;
    if !resp.status().is_success() {
        return Err(io::Error::other(format!(
            "OIDC JWKS request failed with status {}",
            resp.status()
        )));
    }
    resp.json::<JwkSet>()
        .await
        .map_err(|err| io::Error::other(format!("failed to parse OIDC JWKS: {err}")))
}

fn require_trusted_url(name: &str, value: &str) -> io::Result<()> {
    let url = url::Url::parse(value).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("OIDC {name} is not a valid URL: {err}"),
        )
    })?;
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_loopback_url(&url) => Ok(()),
        scheme => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("OIDC {name} must use HTTPS, got {scheme}"),
        )),
    }
}

fn is_loopback_url(url: &url::Url) -> bool {
    matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}

fn parse_allowed_algorithm(value: &str) -> Option<Algorithm> {
    match value {
        "RS256" => Some(Algorithm::RS256),
        "ES256" => Some(Algorithm::ES256),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use jsonwebtoken::EncodingKey;
    use jsonwebtoken::Header;
    use serde_json::json;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    const CLIENT_ID: &str = "sakrylle-cli";
    const NONCE: &str = "nonce-123";

    const ES256_KID: &str = "es256-test-key";

    // Real P-256 private key (PKCS#8), test-only.
    const TEST_EC_PRIVATE_KEY_PEM: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgSDKKdXs1zAc8PT1U
J9F2jGuJosAvKOsfvG/RD7EHYwihRANCAASAtsayQTFIuklZWia6MtUuHIFhvPVK
fU5UloEKyhNtleNJcA/rW8ZLVY334pB0n8lHwMCuhojRPbeEff5UclTu
-----END PRIVATE KEY-----"#;

    fn es256_jwks_body() -> serde_json::Value {
        json!({
            "keys": [{
                "kty": "EC",
                "crv": "P-256",
                "alg": "ES256",
                "use": "sig",
                "kid": ES256_KID,
                "x": "gLbGskExSLpJWVomujLVLhyBYbz1Sn1OVJaBCsoTbZU",
                "y": "40lwD-tbxktVjffikHSfyUfAwK6GiNE9t4R9_lRyVO4"
            }]
        })
    }

    fn es256_discovery(server: &MockServer) -> OidcDiscovery {
        let mut d = discovery(server);
        d.id_token_signing_alg_values_supported = Some(vec!["ES256".to_string()]);
        d
    }

    async fn mount_es256_jwks(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/oauth/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(es256_jwks_body()))
            .mount(server)
            .await;
    }

    fn signed_es256_id_token(
        issuer: &str,
        audience: serde_json::Value,
        nonce: Option<&str>,
    ) -> jsonwebtoken::errors::Result<String> {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(ES256_KID.to_string());
        let now = Utc::now().timestamp() as usize;
        let mut claims = json!({
            "iss": issuer,
            "sub": "user-123",
            "aud": audience,
            "iat": now,
            "exp": now + 3600,
        });
        if let Some(nonce) = nonce {
            claims["nonce"] = json!(nonce);
        }
        jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_ec_pem(TEST_EC_PRIVATE_KEY_PEM)?,
        )
    }

    fn discovery(server: &MockServer) -> OidcDiscovery {
        OidcDiscovery {
            issuer: server.uri(),
            authorization_endpoint: format!("{}/oauth/authorize", server.uri()),
            token_endpoint: format!("{}/oauth/token", server.uri()),
            userinfo_endpoint: None,
            jwks_uri: format!("{}/oauth/jwks", server.uri()),
            device_authorization_endpoint: Some(format!("{}/oauth/device/code", server.uri())),
            end_session_endpoint: None,
            code_challenge_methods_supported: Some(vec!["S256".to_string()]),
            id_token_signing_alg_values_supported: Some(vec!["RS256".to_string()]),
        }
    }

    async fn mount_jwks(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/oauth/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(test_jwks_body()))
            .mount(server)
            .await;
    }

    fn signed_id_token(
        issuer: &str,
        audience: serde_json::Value,
        nonce: Option<&str>,
    ) -> jsonwebtoken::errors::Result<String> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key".to_string());
        let now = Utc::now().timestamp() as usize;
        let mut claims = json!({
            "iss": issuer,
            "sub": "user-123",
            "aud": audience,
            "iat": now,
            "exp": now + 3600,
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "workspace-123"
            }
        });
        if let Some(nonce) = nonce {
            claims["nonce"] = json!(nonce);
        }
        jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM)?,
        )
    }

    #[tokio::test]
    async fn verifies_valid_rs256_id_token() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);
        let token = signed_id_token(&discovery.issuer, json!(CLIENT_ID), Some(NONCE))
            .expect("signed token");

        let verified = verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect("valid token should verify");

        assert_eq!(verified.raw, token);
        assert_eq!(verified.claims.sub, "user-123");
        assert_eq!(verified.claims.iss, discovery.issuer);
        assert!(verified.claims.exp > 0);
        assert!(verified.claims.nbf.is_none());
        assert!(
            verified
                .claims
                .extra
                .contains_key("https://api.openai.com/auth")
        );
    }

    #[tokio::test]
    async fn rejects_wrong_issuer() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);
        let token = signed_id_token("https://evil.example", json!(CLIENT_ID), Some(NONCE))
            .expect("signed token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("wrong issuer should fail");
    }

    #[tokio::test]
    async fn rejects_wrong_audience() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);
        let token = signed_id_token(&discovery.issuer, json!("other-client"), Some(NONCE))
            .expect("signed token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("wrong audience should fail");
    }

    #[tokio::test]
    async fn rejects_missing_nonce_for_browser_login() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);
        let token =
            signed_id_token(&discovery.issuer, json!(CLIENT_ID), None).expect("signed token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("missing nonce should fail");
    }

    #[test]
    fn rejects_discovery_issuer_mismatch() {
        let discovery = OidcDiscovery {
            issuer: "https://issuer.example".to_string(),
            authorization_endpoint: "https://issuer.example/oauth/authorize".to_string(),
            token_endpoint: "https://issuer.example/oauth/token".to_string(),
            userinfo_endpoint: None,
            jwks_uri: "https://issuer.example/oauth/jwks".to_string(),
            device_authorization_endpoint: None,
            end_session_endpoint: None,
            code_challenge_methods_supported: Some(vec!["S256".to_string()]),
            id_token_signing_alg_values_supported: Some(vec!["RS256".to_string()]),
        };

        validate_discovery(&discovery, "https://other.example", false)
            .expect_err("issuer mismatch should fail");
    }

    fn test_jwks_body() -> serde_json::Value {
        json!({
            "keys": [{
                "kty": "RSA",
                "kid": "test-key",
                "use": "sig",
                "alg": "RS256",
                "n": "1qQF2MqTrGAMDm7wXbjJP5sWqGA83tAGUs2ksy7iJXLJdhCg4AtwGm4SFl4f6kxhCSzlN1QdXuZjvRT2wZZiGUi9xUE28rf4WLrTxSnwqLuTy5knMP08yC0t_0YU_FGPZMcWb14hG05IvZr8UbmRaVagxSR8H4rSIymRoVwwmFSrqz068XrWGSYNIfLEASyo5GdAaqmk1JALINHgYGQJVxMxtwcvDxoVKmC7eltUNymMNBZhsv4E8sx9YNLpBoEibznfEpDU_DGzrM5eZCsQzaqbhBOlGd427ifud_Nnd9cPqzgCUc23-0FXSPfpbgksCXAwAmD0OFjQWrgqVdKL6Q",
                "e": "AQAB",
            }]
        })
    }

    const TEST_RSA_PRIVATE_KEY_PEM: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDWpAXYypOsYAwO
bvBduMk/mxaoYDze0AZSzaSzLuIlcsl2EKDgC3AabhIWXh/qTGEJLOU3VB1e5mO9
FPbBlmIZSL3FQTbyt/hYutPFKfCou5PLmScw/TzILS3/RhT8UY9kxxZvXiEbTki9
mvxRuZFpVqDFJHwfitIjKZGhXDCYVKurPTrxetYZJg0h8sQBLKjkZ0BqqaTUkAsg
0eBgZAlXEzG3By8PGhUqYLt6W1Q3KYw0FmGy/gTyzH1g0ukGgSJvOd8SkNT8MbOs
zl5kKxDNqpuEE6UZ3jbuJ+5382d31w+rOAJRzbf7QVdI9+luCSwJcDACYPQ4WNBa
uCpV0ovpAgMBAAECggEAVu84LwZdqYN9XpswX8VoPYrjMm9IODapWQBRpQFoNyK2
1ksF3bjEPvA2Azk8U/l7k+vLKw22l6lY3EyRZPcz5GnB8xLm3ogE3mtNOp4yCyVu
RxhQ91aaN7mU17/a4BdorLi2LYVCg3zBmYociD1Q2AluNGsCmwPu+K7tfR2J0Sg8
NjqiTbDG1XDpR/icwgC9t6vh8lZpCHDhF4tbQfLLVLeA/OdcuzXDyMCXbmdVIdBQ
rm4aIFmr2e1/2ctTbCg85S6AGFTH+pSLjrwTzyvf+F6NW5uNjLQAQLFj+EznBDxj
Xdx90cySrjsKK6PVWQF4RiTvkSW8eWL7R6B2FZbGwQKBgQDuVQRj72hWloR7mbEL
aUEEv3pIXTMXWEsoMBNczos/1L1RnAN1AI44TurznasPZAWvQj+kVbLDR+TAeZrL
iA8HIWswQUI18hFmgKzSkwIXGtubcKVrgsKeS4lMDKCM/Ef6WAYdeq6ronoY5lCN
YrJFmGp81W5zcV7lyiycgbSiGwKBgQDmjWYf6pZjrK7Z+OJ3X1AZfi2vss15SCvL
3fPgzIDbViztpGyQhc3DQZIsBNIu0xZp/veGce9TEeTds2ro9NfdJFeou8+fC7Pq
sOsM3amGFFi+ZW/9BWyjZEM88bgWWAjqLHbpfHDxjAf5CSxddqxgHlbP0Ytyb1Vg
gmPDn9YKSwKBgQDbTi3hC35WFuDHn0/zcSHcDZmnFuOZeqyFyV83yfMGhGrEuqvP
sPgtRikajJ3IZsB4WZyYSidZXEFY/0z6NjOl2xF38MTNQPbT/FmK1q1Yt2UWrlv5
BvSwlk87RG9D7C0LZo4R+D7cPoDdgqjiwMvMEIkEX5zn641oI1ZTmWKuuwKBgQCD
KF+3unnRvHRAVoFnTZbA2fJdqMeRvogD04GhGlYX8V9f1hFY6nXTJaNlXVzA/J8c
r8ra9kgjJuPfZ+ljG58OFFW2DRohLcQtuHYPfK6rMzoFHqnl9EcIcMp7ijuionR3
29HOJFgQYgxLFXfit9d6WugiE+BTupiEbckZif13HwKBgE/lAlkVHP6YahOO2Ljc
J1bwkqKZTB5dHolX9A58e/xXnfZ5P8f3Z83+Izap3FwqQulk7b1WO1MQcHuVg2NN
5da0D4h2rYOXnbYIg0BVu4spQbaM6ewsp66b8+MzLOBvj8SzWdt1Oyw0q/MRyQAR
8U4M2TSWCKUY/A6sT4W8+mT9
-----END PRIVATE KEY-----"#;

    #[tokio::test]
    async fn verifies_valid_es256_id_token() {
        let server = MockServer::start().await;
        mount_es256_jwks(&server).await;
        let discovery = es256_discovery(&server);
        let token = signed_es256_id_token(&discovery.issuer, json!(CLIENT_ID), Some(NONCE))
            .expect("signed es256 token");

        let verified = verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect("valid ES256 token should verify");

        assert_eq!(verified.claims.sub, "user-123");
        assert_eq!(verified.claims.iss, discovery.issuer);
    }

    #[tokio::test]
    async fn rejects_hs256_id_token() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);

        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("test-key".to_string());
        let now = Utc::now().timestamp() as usize;
        let claims = json!({
            "iss": discovery.issuer,
            "sub": "user-123",
            "aud": CLIENT_ID,
            "iat": now,
            "exp": now + 3600,
            "nonce": NONCE,
        });
        let token = jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"shared-secret"),
        )
        .expect("signed hs256 token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("HS256 is not an allowed id_token algorithm");
    }

    #[tokio::test]
    async fn rejects_nonce_mismatch() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);
        let token = signed_id_token(&discovery.issuer, json!(CLIENT_ID), Some("other-nonce"))
            .expect("signed token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("mismatched nonce should fail");
    }
}
