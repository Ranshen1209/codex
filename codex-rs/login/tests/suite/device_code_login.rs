#![allow(clippy::unwrap_used)]

use anyhow::Context;
use codex_config::types::AuthCredentialsStoreMode;
use codex_login::ServerOptions;
use codex_login::auth::load_auth_dot_json;
use codex_login::run_device_code_login;
use core_test_support::skip_if_no_network;
use jsonwebtoken::Algorithm;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tempfile::tempdir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::Request;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

const WORKSPACE_ID_ALLOWED: &str = "123e4567-e89b-42d3-a456-426614174000";
const WORKSPACE_ID_DISALLOWED: &str = "123e4567-e89b-42d3-a456-426614174002";

async fn mock_discovery(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": server.uri(),
            "authorization_endpoint": format!("{}/oauth/authorize", server.uri()),
            "token_endpoint": format!("{}/oauth/token", server.uri()),
            "jwks_uri": format!("{}/oauth/jwks", server.uri()),
            "device_authorization_endpoint": format!("{}/oauth/device/code", server.uri()),
            "code_challenge_methods_supported": ["S256"],
            "id_token_signing_alg_values_supported": ["RS256"],
        })))
        .mount(server)
        .await;
}

async fn mock_jwks(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/oauth/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(test_jwks_body()))
        .mount(server)
        .await;
}

async fn mock_device_code_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/oauth/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "device-code-123",
            "user_code": "CODE-12345",
            "verification_uri": format!("{}/activate", server.uri()),
            "interval": "0",
        })))
        .mount(server)
        .await;
}

async fn mock_device_code_failure(server: &MockServer, status: u16) {
    Mock::given(method("POST"))
        .and(path("/oauth/device/code"))
        .respond_with(ResponseTemplate::new(status))
        .mount(server)
        .await;
}

async fn mock_token_success(server: &MockServer, jwt: String) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id_token": jwt,
            "access_token": "access-token-123",
            "refresh_token": "refresh-token-123",
            "token_type": "Bearer",
            "expires_in": 3600,
        })))
        .mount(server)
        .await;
}

async fn mock_token_sequence(
    server: &MockServer,
    counter: Arc<AtomicUsize>,
    first_response: ResponseTemplate,
    jwt: String,
) {
    let c = counter.clone();
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(move |_: &Request| {
            let attempt = c.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                first_response.clone()
            } else {
                ResponseTemplate::new(200).set_body_json(json!({
                    "id_token": jwt,
                    "access_token": "access-token-123",
                    "refresh_token": "refresh-token-123",
                }))
            }
        })
        .expect(2)
        .mount(server)
        .await;
}

async fn mock_token_error(server: &MockServer, status: u16, error: &str, description: &str) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(json!({
            "error": error,
            "error_description": description,
        })))
        .mount(server)
        .await;
}

fn server_opts(
    codex_home: &tempfile::TempDir,
    issuer: String,
    cli_auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> ServerOptions {
    let mut opts = ServerOptions::new(
        codex_home.path().to_path_buf(),
        codex_login::CLIENT_ID.to_string(),
        /*forced_chatgpt_workspace_id*/ None,
        cli_auth_credentials_store_mode,
    );
    opts.issuer = issuer;
    opts.open_browser = false;
    opts
}

fn signed_id_token(issuer: &str, chatgpt_account_id: Option<&str>) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-key".to_string());
    let now = chrono::Utc::now().timestamp() as usize;
    let mut claims = json!({
        "iss": issuer,
        "sub": "user-123",
        "aud": codex_login::CLIENT_ID,
        "iat": now,
        "exp": now + 3600,
        "email": "user@example.com",
    });
    if let Some(chatgpt_account_id) = chatgpt_account_id {
        claims["https://api.openai.com/auth"] = json!({
            "chatgpt_plan_type": "pro",
            "chatgpt_account_id": chatgpt_account_id,
        });
    }

    jsonwebtoken::encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM).unwrap(),
    )
    .unwrap()
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
async fn device_code_login_integration_succeeds() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let codex_home = tempdir()?;
    let mock_server = MockServer::start().await;
    mock_discovery(&mock_server).await;
    mock_jwks(&mock_server).await;
    mock_device_code_success(&mock_server).await;
    let jwt = signed_id_token(&mock_server.uri(), Some(WORKSPACE_ID_ALLOWED));
    mock_token_success(&mock_server, jwt.clone()).await;

    run_device_code_login(server_opts(
        &codex_home,
        mock_server.uri(),
        AuthCredentialsStoreMode::File,
    ))
    .await
    .expect("device code login integration should succeed");

    let auth = load_auth_dot_json(codex_home.path(), AuthCredentialsStoreMode::File)?
        .context("auth.json written")?;
    assert!(auth.openai_api_key.is_none());
    let tokens = auth.tokens.expect("tokens persisted");
    assert_eq!(tokens.access_token, "access-token-123");
    assert_eq!(tokens.refresh_token, "refresh-token-123");
    assert_eq!(tokens.id_token.raw_jwt, jwt);
    assert_eq!(tokens.account_id.as_deref(), Some(WORKSPACE_ID_ALLOWED));
    Ok(())
}

#[tokio::test]
async fn device_code_login_rejects_workspace_mismatch() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let codex_home = tempdir()?;
    let mock_server = MockServer::start().await;
    mock_discovery(&mock_server).await;
    mock_jwks(&mock_server).await;
    mock_device_code_success(&mock_server).await;
    let jwt = signed_id_token(&mock_server.uri(), Some(WORKSPACE_ID_DISALLOWED));
    mock_token_success(&mock_server, jwt).await;

    let mut opts = server_opts(
        &codex_home,
        mock_server.uri(),
        AuthCredentialsStoreMode::File,
    );
    opts.forced_chatgpt_workspace_id = Some(vec![WORKSPACE_ID_ALLOWED.to_string()]);

    let err = run_device_code_login(opts)
        .await
        .expect_err("device code login should fail when workspace mismatches");
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);

    let auth = load_auth_dot_json(codex_home.path(), AuthCredentialsStoreMode::File)?;
    assert!(
        auth.is_none(),
        "auth.json should not be created when workspace validation fails"
    );
    Ok(())
}

#[tokio::test]
async fn device_code_login_integration_handles_usercode_http_failure() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let codex_home = tempdir()?;
    let mock_server = MockServer::start().await;
    mock_discovery(&mock_server).await;
    mock_device_code_failure(&mock_server, /*status*/ 503).await;

    let err = run_device_code_login(server_opts(
        &codex_home,
        mock_server.uri(),
        AuthCredentialsStoreMode::File,
    ))
    .await
    .expect_err("usercode HTTP failure should bubble up");
    assert!(
        err.to_string()
            .contains("device code request failed with status"),
        "unexpected error: {err:?}"
    );

    let auth = load_auth_dot_json(codex_home.path(), AuthCredentialsStoreMode::File)?;
    assert!(auth.is_none(), "auth.json should not be created when login fails");
    Ok(())
}

#[tokio::test]
async fn device_code_login_persists_without_api_key_or_workspace_claim() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let codex_home = tempdir()?;
    let mock_server = MockServer::start().await;
    mock_discovery(&mock_server).await;
    mock_jwks(&mock_server).await;
    mock_device_code_success(&mock_server).await;
    let jwt = signed_id_token(&mock_server.uri(), None);
    mock_token_success(&mock_server, jwt.clone()).await;

    run_device_code_login(server_opts(
        &codex_home,
        mock_server.uri(),
        AuthCredentialsStoreMode::File,
    ))
    .await
    .expect("device login should succeed without API key exchange");

    let auth = load_auth_dot_json(codex_home.path(), AuthCredentialsStoreMode::File)?
        .context("auth.json written")?;
    assert!(auth.openai_api_key.is_none());
    let tokens = auth.tokens.expect("tokens persisted");
    assert_eq!(tokens.access_token, "access-token-123");
    assert_eq!(tokens.refresh_token, "refresh-token-123");
    assert_eq!(tokens.id_token.raw_jwt, jwt);
    assert_eq!(tokens.account_id, None);
    Ok(())
}

#[tokio::test]
async fn device_code_login_continues_after_authorization_pending() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let codex_home = tempdir()?;
    let mock_server = MockServer::start().await;
    mock_discovery(&mock_server).await;
    mock_jwks(&mock_server).await;
    mock_device_code_success(&mock_server).await;
    let jwt = signed_id_token(&mock_server.uri(), Some(WORKSPACE_ID_ALLOWED));
    mock_token_sequence(
        &mock_server,
        Arc::new(AtomicUsize::new(0)),
        ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_pending"
        })),
        jwt,
    )
    .await;

    run_device_code_login(server_opts(
        &codex_home,
        mock_server.uri(),
        AuthCredentialsStoreMode::File,
    ))
    .await
    .expect("device login should keep polling while authorization is pending");
    Ok(())
}

#[tokio::test]
async fn device_code_login_fails_on_terminal_oauth_error() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));

    let codex_home = tempdir()?;
    let mock_server = MockServer::start().await;
    mock_discovery(&mock_server).await;
    mock_device_code_success(&mock_server).await;
    mock_token_error(&mock_server, 400, "access_denied", "Denied").await;

    let err = run_device_code_login(server_opts(
        &codex_home,
        mock_server.uri(),
        AuthCredentialsStoreMode::File,
    ))
    .await
    .expect_err("terminal device auth errors should fail");
    assert!(
        err.to_string().contains("device auth failed: Denied"),
        "unexpected error: {err:?}"
    );

    let auth = load_auth_dot_json(codex_home.path(), AuthCredentialsStoreMode::File)?;
    assert!(
        auth.is_none(),
        "auth.json should not be created when device auth fails"
    );
    Ok(())
}
