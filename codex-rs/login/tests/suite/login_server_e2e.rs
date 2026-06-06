#![allow(clippy::unwrap_used)]
use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::thread;

use anyhow::Result;
use codex_config::types::AuthCredentialsStoreMode;
use codex_login::ServerOptions;
use codex_login::run_login_server;
use core_test_support::skip_if_no_network;
use pretty_assertions::assert_eq;
use tempfile::tempdir;
use url::Url;

const DEFAULT_LOGIN_PORT: u16 = 1455;
const WORKSPACE_ID_ALLOWED: &str = "123e4567-e89b-42d3-a456-426614174000";
const WORKSPACE_ID_SECOND_ALLOWED: &str = "123e4567-e89b-42d3-a456-426614174001";
const WORKSPACE_ID_DISALLOWED: &str = "123e4567-e89b-42d3-a456-426614174002";

// See spawn.rs for details

fn start_mock_issuer(chatgpt_account_id: &str) -> (SocketAddr, thread::JoinHandle<()>) {
    // Bind to a random available port
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let issuer = format!("http://{}:{}", addr.ip(), addr.port());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let chatgpt_account_id = chatgpt_account_id.to_string();

    let handle = thread::spawn(move || {
        while let Ok(mut req) = server.recv() {
            let url = req.url().to_string();
            if url.starts_with("/.well-known/openid-configuration") {
                let body = serde_json::json!({
                    "issuer": issuer,
                    "authorization_endpoint": format!("{issuer}/oauth/authorize"),
                    "token_endpoint": format!("{issuer}/oauth/token"),
                    "jwks_uri": format!("{issuer}/oauth/jwks"),
                    "code_challenge_methods_supported": ["S256"],
                    "id_token_signing_alg_values_supported": ["RS256"],
                });
                respond_json(req, body);
            } else if url.starts_with("/oauth/jwks") {
                respond_json(req, test_jwks_body());
            } else if url.starts_with("/oauth/token") {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let code = url::form_urlencoded::parse(body.as_bytes())
                    .find_map(|(key, value)| (key == "code").then(|| value.into_owned()))
                    .unwrap_or_default();
                let id_token = signed_id_token(&issuer, &chatgpt_account_id, &code);
                let tokens = serde_json::json!({
                    "id_token": id_token,
                    "access_token": "access-123",
                    "refresh_token": "refresh-123",
                });
                respond_json(req, tokens);
            } else {
                let _ = req
                    .respond(tiny_http::Response::from_string("not found").with_status_code(404));
            }
        }
    });

    (addr, handle)
}

fn respond_json(req: tiny_http::Request, body: serde_json::Value) {
    let data = serde_json::to_vec(&body).unwrap();
    let mut resp = tiny_http::Response::from_data(data);
    resp.add_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
            .unwrap_or_else(|_| panic!("header bytes")),
    );
    let _ = req.respond(resp);
}

fn signed_id_token(issuer: &str, chatgpt_account_id: &str, nonce: &str) -> String {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("test-key".to_string());
    let now = chrono::Utc::now().timestamp() as usize;
    jsonwebtoken::encode(
        &header,
        &serde_json::json!({
            "iss": issuer,
            "sub": "user-123",
            "aud": codex_login::CLIENT_ID,
            "iat": now,
            "exp": now + 3600,
            "nonce": nonce,
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_plan_type": "pro",
                "chatgpt_account_id": chatgpt_account_id,
            }
        }),
        &jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_KEY_PEM).unwrap(),
    )
    .unwrap()
}

fn nonce_from_auth_url(auth_url: &str) -> Result<String> {
    let auth_url = Url::parse(auth_url)?;
    auth_url
        .query_pairs()
        .find_map(|(key, value)| (key == "nonce").then(|| value.into_owned()))
        .ok_or_else(|| anyhow::anyhow!("auth URL should include nonce"))
}

fn test_jwks_body() -> serde_json::Value {
    serde_json::json!({
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
async fn end_to_end_login_flow_persists_auth_json() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let chatgpt_account_id = "12345678-0000-0000-0000-000000000000";
    let (issuer_addr, issuer_handle) = start_mock_issuer(chatgpt_account_id);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let codex_home = tmp.path().to_path_buf();

    // Seed auth.json with stale API key + tokens that should be overwritten.
    let stale_auth = serde_json::json!({
        "OPENAI_API_KEY": "sk-stale",
        "tokens": {
            "id_token": "stale.header.payload",
            "access_token": "stale-access",
            "refresh_token": "stale-refresh",
            "account_id": "stale-acc"
        }
    });
    std::fs::write(
        codex_home.join("auth.json"),
        serde_json::to_string_pretty(&stale_auth)?,
    )?;

    let state = "test_state_123".to_string();

    // Run server in background
    let server_home = codex_home.clone();

    let opts = ServerOptions {
        codex_home: server_home,
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some(state),
        forced_chatgpt_workspace_id: Some(vec![chatgpt_account_id.to_string()]),
        codex_streamlined_login: false,
    };
    let server = run_login_server(opts).await?;
    assert!(
        server
            .auth_url
            .contains(format!("allowed_workspace_id={chatgpt_account_id}").as_str()),
        "auth URL should include forced workspace parameter"
    );
    let login_port = server.actual_port;
    let nonce = nonce_from_auth_url(&server.auth_url)?;

    // Simulate browser callback, and follow redirect to /success
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let url = format!("http://127.0.0.1:{login_port}/auth/callback?code={nonce}&state=test_state_123");
    let resp = client.get(&url).send().await?;
    assert!(resp.status().is_success());

    // Wait for server shutdown
    server.block_until_done().await?;

    // Validate auth.json
    let auth_path = codex_home.join("auth.json");
    let data = std::fs::read_to_string(&auth_path)?;
    let json: serde_json::Value = serde_json::from_str(&data)?;
    assert!(
        json.get("OPENAI_API_KEY").is_none_or(serde_json::Value::is_null),
        "OIDC login should not persist an API-key alias"
    );
    assert_eq!(json["tokens"]["access_token"], "access-123");
    assert_eq!(json["tokens"]["refresh_token"], "refresh-123");
    assert_eq!(json["tokens"]["account_id"], chatgpt_account_id);

    // Stop mock issuer
    drop(issuer_handle);
    Ok(())
}

#[tokio::test]
async fn creates_missing_codex_home_dir() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_ALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let codex_home = tmp.path().join("missing-subdir"); // does not exist

    let state = "state2".to_string();

    // Run server in background
    let server_home = codex_home.clone();
    let opts = ServerOptions {
        codex_home: server_home,
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some(state),
        forced_chatgpt_workspace_id: None,
        codex_streamlined_login: false,
    };
    let server = run_login_server(opts).await?;
    let login_port = server.actual_port;
    let nonce = nonce_from_auth_url(&server.auth_url)?;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{login_port}/auth/callback?code={nonce}&state=state2");
    let resp = client.get(&url).send().await?;
    assert!(resp.status().is_success());

    server.block_until_done().await?;

    let auth_path = codex_home.join("auth.json");
    assert!(
        auth_path.exists(),
        "auth.json should be created even if parent dir was missing"
    );
    Ok(())
}

#[tokio::test]
async fn login_server_includes_forced_workspaces_as_one_query_param() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_ALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let codex_home = tmp.path().to_path_buf();
    let state = "state-multi".to_string();

    let opts = ServerOptions {
        codex_home,
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some(state),
        forced_chatgpt_workspace_id: Some(vec![
            WORKSPACE_ID_ALLOWED.to_string(),
            WORKSPACE_ID_SECOND_ALLOWED.to_string(),
        ]),
        codex_streamlined_login: false,
    };
    let server = run_login_server(opts).await?;
    let auth_url = Url::parse(&server.auth_url)?;
    let allowed_workspace_ids = auth_url
        .query_pairs()
        .filter_map(|(key, value)| (key == "allowed_workspace_id").then(|| value.into_owned()))
        .collect::<Vec<_>>();
    assert_eq!(
        allowed_workspace_ids,
        vec![format!(
            "{WORKSPACE_ID_ALLOWED},{WORKSPACE_ID_SECOND_ALLOWED}"
        )]
    );

    Ok(())
}

#[tokio::test]
async fn forced_chatgpt_workspace_id_mismatch_blocks_login() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_DISALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let codex_home = tmp.path().to_path_buf();
    let state = "state-mismatch".to_string();

    let opts = ServerOptions {
        codex_home: codex_home.clone(),
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some(state.clone()),
        forced_chatgpt_workspace_id: Some(vec![WORKSPACE_ID_ALLOWED.to_string()]),
        codex_streamlined_login: false,
    };
    let server = run_login_server(opts).await?;
    assert!(
        server
            .auth_url
            .contains(&format!("allowed_workspace_id={WORKSPACE_ID_ALLOWED}")),
        "auth URL should include forced workspace parameter"
    );
    let login_port = server.actual_port;
    let nonce = nonce_from_auth_url(&server.auth_url)?;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{login_port}/auth/callback?code={nonce}&state={state}");
    let resp = client.get(&url).send().await?;
    assert!(resp.status().is_success());
    let body = resp.text().await?;
    assert!(
        body.contains(&format!(
            "Login is restricted to workspace id(s) {WORKSPACE_ID_ALLOWED}"
        )),
        "error body should mention workspace restriction"
    );

    let result = server.block_until_done().await;
    assert!(
        result.is_err(),
        "login should fail due to workspace mismatch"
    );
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

    let auth_path = codex_home.join("auth.json");
    assert!(
        !auth_path.exists(),
        "auth.json should not be written when the workspace mismatches"
    );

    Ok(())
}

#[tokio::test]
async fn oauth_access_denied_missing_entitlement_blocks_login_with_clear_error() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_ALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let codex_home = tmp.path().to_path_buf();
    let state = "state-entitlement".to_string();

    let opts = ServerOptions {
        codex_home: codex_home.clone(),
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some(state.clone()),
        forced_chatgpt_workspace_id: None,
        codex_streamlined_login: false,
    };
    let server = run_login_server(opts).await?;
    let login_port = server.actual_port;

    let client = reqwest::Client::new();
    let url = format!(
        "http://127.0.0.1:{login_port}/auth/callback?state={state}&error=access_denied&error_description=missing_codex_entitlement"
    );
    let resp = client.get(&url).send().await?;
    assert!(resp.status().is_success());
    let body = resp.text().await?;
    assert!(
        body.contains("Sakrylle is not enabled for your workspace"),
        "error body should clearly explain the Sakrylle access denial"
    );
    assert!(
        body.contains("Contact your workspace administrator"),
        "error body should tell the user how to get access"
    );
    assert!(
        body.contains("access_denied"),
        "error body should still include the oauth error code"
    );
    assert!(
        !body.contains("missing_codex_entitlement"),
        "known entitlement errors should be mapped to user-facing copy"
    );

    let result = server.block_until_done().await;
    assert!(result.is_err(), "login should fail for access_denied");
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    assert!(
        err.to_string()
            .contains("Contact your workspace administrator"),
        "terminal error should also tell the user what to do next"
    );

    let auth_path = codex_home.join("auth.json");
    assert!(
        !auth_path.exists(),
        "auth.json should not be written when oauth callback is denied"
    );

    Ok(())
}

#[tokio::test]
async fn oauth_access_denied_unknown_reason_uses_generic_error_page() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_ALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let codex_home = tmp.path().to_path_buf();
    let state = "state-generic-denial".to_string();

    let opts = ServerOptions {
        codex_home: codex_home.clone(),
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: 0,
        open_browser: false,
        force_state: Some(state.clone()),
        forced_chatgpt_workspace_id: None,
        codex_streamlined_login: false,
    };
    let server = run_login_server(opts).await?;
    let login_port = server.actual_port;

    let client = reqwest::Client::new();
    let url = format!(
        "http://127.0.0.1:{login_port}/auth/callback?state={state}&error=access_denied&error_description=some_other_reason"
    );
    let resp = client.get(&url).send().await?;
    assert!(resp.status().is_success());
    let body = resp.text().await?;
    assert!(
        body.contains("Sign-in could not be completed"),
        "generic oauth denial should use the generic error page title"
    );
    assert!(
        body.contains("Sign-in failed: some_other_reason"),
        "generic oauth denial should preserve the oauth error details"
    );
    assert!(
        body.contains("Return to the terminal and try again"),
        "generic oauth denial should keep the generic help text"
    );
    assert!(
        body.contains("access_denied"),
        "generic oauth denial should include the oauth error code"
    );
    assert!(
        body.contains("some_other_reason"),
        "generic oauth denial should include the oauth error description"
    );
    assert!(
        !body.contains("Sakrylle is not enabled for your workspace"),
        "generic oauth denial should not show the entitlement-specific title"
    );
    assert!(
        !body.contains("request access to Sakrylle"),
        "generic oauth denial should not show the entitlement-specific admin guidance"
    );

    let result = server.block_until_done().await;
    assert!(result.is_err(), "login should fail for access_denied");
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    assert!(
        err.to_string()
            .contains("Sign-in failed: some_other_reason"),
        "terminal error should preserve generic oauth details"
    );

    let auth_path = codex_home.join("auth.json");
    assert!(
        !auth_path.exists(),
        "auth.json should not be written when oauth callback is denied"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn uses_random_port_when_default_port_is_in_use() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let default_port_listener = match TcpListener::bind(("127.0.0.1", DEFAULT_LOGIN_PORT)) {
        Ok(listener) => listener,
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            eprintln!("Skipping test because 127.0.0.1:{DEFAULT_LOGIN_PORT} is already in use");
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_ALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());
    let tmp = tempdir()?;

    let mut opts = ServerOptions::new(
        tmp.path().to_path_buf(),
        codex_login::CLIENT_ID.to_string(),
        /*forced_chatgpt_workspace_id*/ None,
        AuthCredentialsStoreMode::File,
    );
    opts.issuer = issuer;
    opts.open_browser = false;
    opts.force_state = Some("random_port_state".to_string());

    let server = run_login_server(opts).await?;
    let actual_port = server.actual_port;
    let auth_url = server.auth_url.clone();
    server.cancel();
    let _ = server.block_until_done().await;
    drop(default_port_listener);

    assert_ne!(actual_port, DEFAULT_LOGIN_PORT);
    assert!(auth_url.contains(&format!(
        "redirect_uri=http%3A%2F%2F127.0.0.1%3A{actual_port}%2Fcallback"
    )));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ignores_requested_port_and_uses_random_available_port() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let requested_port_listener = match TcpListener::bind(("127.0.0.1", DEFAULT_LOGIN_PORT)) {
        Ok(listener) => listener,
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            eprintln!("Skipping test because 127.0.0.1:{DEFAULT_LOGIN_PORT} is already in use");
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    let (issuer_addr, _issuer_handle) = start_mock_issuer(WORKSPACE_ID_ALLOWED);
    let issuer = format!("http://{}:{}", issuer_addr.ip(), issuer_addr.port());

    let tmp = tempdir()?;
    let mut opts = ServerOptions {
        codex_home: tmp.path().to_path_buf(),
        cli_auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        client_id: codex_login::CLIENT_ID.to_string(),
        issuer,
        port: DEFAULT_LOGIN_PORT,
        open_browser: false,
        force_state: Some("requested_port_state".to_string()),
        forced_chatgpt_workspace_id: None,
        codex_streamlined_login: false,
    };

    let server = run_login_server(opts.clone()).await?;
    assert_ne!(server.actual_port, DEFAULT_LOGIN_PORT);
    assert_ne!(server.actual_port, opts.port);

    opts.port = 0;
    server.cancel();
    let _ = server.block_until_done().await;
    drop(requested_port_listener);

    Ok(())
}
