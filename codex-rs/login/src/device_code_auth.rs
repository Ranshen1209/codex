use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::Deserializer;
use serde::de::{self};
use std::time::Duration;
use std::time::Instant;

use crate::oidc::validate_discovery;
use crate::oidc::verify_id_token;
use crate::server::ServerOptions;
use crate::server::ensure_workspace_allowed;
use crate::server::fetch_discovery;
use codex_client::build_reqwest_client_with_custom_ca;
use std::io;

const ANSI_BLUE: &str = "\x1b[94m";
const ANSI_GRAY: &str = "\x1b[90m";
const ANSI_RESET: &str = "\x1b[0m";

/// SAKRYLLE: OIDC login — device code info per RFC 8628.
#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub verification_url: String,
    pub user_code: String,
    device_code: String,
    interval: u64,
}

/// SAKRYLLE: OIDC login — RFC 8628 device authorization response.
#[derive(Deserialize)]
struct UserCodeResp {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default, deserialize_with = "deserialize_interval")]
    interval: u64,
    #[serde(default, rename = "expires_in")]
    _expires_in: Option<u64>,
}

// SAKRYLLE: OIDC login — request structs removed; using form-encoded body directly per RFC 8628.

fn deserialize_interval<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Interval {
        String(String),
        Number(u64),
    }

    match Interval::deserialize(deserializer)? {
        Interval::String(value) => value.trim().parse::<u64>().map_err(de::Error::custom),
        Interval::Number(value) => Ok(value),
    }
}

/// SAKRYLLE: OIDC login — RFC 8628 token success response.
#[derive(Deserialize)]
struct TokenSuccessResp {
    access_token: String,
    #[serde(default)]
    id_token: Option<String>,
    refresh_token: String,
    #[serde(default, rename = "token_type")]
    _token_type: Option<String>,
    #[serde(default, rename = "expires_in")]
    _expires_in: Option<u64>,
}

/// SAKRYLLE: OIDC login — request the device code via RFC 8628.
/// Uses the discovery `device_authorization_endpoint`.
async fn request_user_code(
    client: &reqwest::Client,
    device_auth_endpoint: &str,
    client_id: &str,
    scope: &str,
) -> std::io::Result<UserCodeResp> {
    let body = format!(
        "client_id={}&scope={}",
        urlencoding::encode(client_id),
        urlencoding::encode(scope),
    );
    let resp = client
        .post(device_auth_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(std::io::Error::other)?;

    if !resp.status().is_success() {
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "device code login is not enabled for this server. Use the browser login instead.",
            ));
        }
        let body_text = resp.text().await.unwrap_or_default();
        return Err(std::io::Error::other(format!(
            "device code request failed with status {status}: {body_text}"
        )));
    }

    let body = resp.text().await.map_err(std::io::Error::other)?;
    serde_json::from_str(&body).map_err(std::io::Error::other)
}

/// SAKRYLLE: OIDC login — poll the token endpoint per RFC 8628 until authorization completes.
async fn poll_for_token(
    client: &reqwest::Client,
    token_endpoint: &str,
    device_code: &str,
    client_id: &str,
    interval: u64,
) -> std::io::Result<TokenSuccessResp> {
    #[derive(Deserialize)]
    struct TokenErrorResp {
        error: String,
        #[serde(default)]
        error_description: Option<String>,
    }

    let max_wait = Duration::from_secs(15 * 60);
    let start = Instant::now();
    let mut interval = interval.max(1);

    loop {
        let body = format!(
            "grant_type={}&device_code={}&client_id={}",
            urlencoding::encode("urn:ietf:params:oauth:grant-type:device_code"),
            urlencoding::encode(device_code),
            urlencoding::encode(client_id),
        );
        let resp = client
            .post(token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(std::io::Error::other)?;

        let status = resp.status();

        if status.is_success() {
            return resp.json().await.map_err(std::io::Error::other);
        }

        let body_text = resp.text().await.unwrap_or_default();
        if status == StatusCode::BAD_REQUEST || status == StatusCode::FORBIDDEN {
            let token_error: TokenErrorResp = serde_json::from_str(&body_text).map_err(|err| {
                std::io::Error::other(format!(
                    "device auth failed with status {status}: failed to parse OAuth error response: {err}"
                ))
            })?;

            match token_error.error.as_str() {
                "authorization_pending" => {}
                "slow_down" => {
                    interval = interval.saturating_add(5);
                }
                "expired_token" | "access_denied" | "invalid_grant" => {
                    let description = token_error
                        .error_description
                        .filter(|description| !description.trim().is_empty())
                        .unwrap_or_else(|| token_error.error.clone());
                    return Err(std::io::Error::other(format!(
                        "device auth failed: {description}"
                    )));
                }
                other => {
                    let description = token_error
                        .error_description
                        .filter(|description| !description.trim().is_empty())
                        .unwrap_or_else(|| other.to_string());
                    return Err(std::io::Error::other(format!(
                        "device auth failed: {description}"
                    )));
                }
            }

            if start.elapsed() >= max_wait {
                return Err(std::io::Error::other(
                    "device auth timed out after 15 minutes",
                ));
            }
            let sleep_for = Duration::from_secs(interval).min(max_wait - start.elapsed());
            tokio::time::sleep(sleep_for).await;
            continue;
        }

        return Err(std::io::Error::other(format!(
            "device auth failed with status {status}: {body_text}"
        )));
    }
}

fn print_device_code_prompt(verification_url: &str, code: &str) {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "\nWelcome to Sakrylle [v{ANSI_GRAY}{version}{ANSI_RESET}]\n{ANSI_GRAY}Sakrylle CLI{ANSI_RESET}\n\
\nFollow these steps to sign in with Sakrylle using device code authorization:\n\
\n1. Open this link in your browser and sign in to your account\n   {ANSI_BLUE}{verification_url}{ANSI_RESET}\n\
\n2. Enter this one-time code {ANSI_GRAY}(expires in 15 minutes){ANSI_RESET}\n   {ANSI_BLUE}{code}{ANSI_RESET}\n\
\n{ANSI_GRAY}Device codes are a common phishing target. Never share this code.{ANSI_RESET}\n",
    );
}

/// SAKRYLLE: OIDC login — request a device code using OIDC discovery.
pub async fn request_device_code(opts: &ServerOptions) -> std::io::Result<DeviceCode> {
    // SAKRYLLE: OIDC login — fetch discovery to get device_authorization_endpoint
    let discovery = fetch_discovery(&opts.issuer).await?;
    validate_discovery(&discovery, &opts.issuer, /*require_device_endpoint*/ true)?;
    let device_auth_endpoint = discovery
        .device_authorization_endpoint
        .as_deref()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "device code login is not supported by this server (no device_authorization_endpoint in discovery)",
            )
        })?;

    let client = build_reqwest_client_with_custom_ca(reqwest::Client::builder())?;
    // SAKRYLLE: OIDC login — use Sakrylle scope
    let scope = crate::server::SAKRYLLE_SCOPE;
    let uc = request_user_code(&client, device_auth_endpoint, &opts.client_id, scope).await?;

    Ok(DeviceCode {
        verification_url: uc.verification_uri,
        user_code: uc.user_code,
        device_code: uc.device_code,
        interval: uc.interval,
    })
}

/// SAKRYLLE: OIDC login — complete the device code login using RFC 8628 token polling.
pub async fn complete_device_code_login(
    opts: ServerOptions,
    device_code: DeviceCode,
) -> std::io::Result<()> {
    // SAKRYLLE: OIDC login — fetch discovery for token endpoint
    let discovery = fetch_discovery(&opts.issuer).await?;
    validate_discovery(&discovery, &opts.issuer, /*require_device_endpoint*/ true)?;
    let client = build_reqwest_client_with_custom_ca(reqwest::Client::builder())?;

    // SAKRYLLE: OIDC login — poll the token endpoint directly (RFC 8628)
    let token_resp = poll_for_token(
        &client,
        &discovery.token_endpoint,
        &device_code.device_code,
        &opts.client_id,
        device_code.interval,
    )
    .await?;

    let id_token = token_resp.id_token.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "OIDC device token response did not include an id_token",
        )
    })?;
    let verified_id_token = verify_id_token(
        &id_token,
        &discovery,
        &opts.client_id,
        /*expected_nonce*/ None,
    )
    .await?;
    if let Err(message) = ensure_workspace_allowed(
        opts.forced_chatgpt_workspace_id.as_deref(),
        &verified_id_token.raw,
    ) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, message));
    }

    // SAKRYLLE: OIDC login — persist tokens directly after strict ID token validation.
    crate::server::persist_tokens_async(
        &opts.codex_home,
        /*api_key*/ None,
        verified_id_token.raw,
        token_resp.access_token,
        token_resp.refresh_token,
        opts.cli_auth_credentials_store_mode,
    )
    .await
}

pub async fn run_device_code_login(opts: ServerOptions) -> std::io::Result<()> {
    let device_code = request_device_code(&opts).await?;
    print_device_code_prompt(&device_code.verification_url, &device_code.user_code);
    complete_device_code_login(opts, device_code).await
}
