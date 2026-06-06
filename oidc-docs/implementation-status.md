---
title: Sakrylle CLI Implementation Status
status: local
scope: product-local
canonical_source: ../../sub2api/sakrylle-docs/10-platform-identity/current-state.md
last_verified: 2026-06-06
---

# Sakrylle CLI Implementation Status

Current status: **Phase 1–2 complete (OIDC validation + device auth fixes). Phase 3–5 remain.**

Canonical platform status lives in [Sakrylle OIDC current state](../../sub2api/sakrylle-docs/10-platform-identity/current-state.md). This file only tracks product-local readiness and gaps.

## Phase 1: OIDC strict RP validation — ✅ Done

- `codex-rs/login/src/oidc.rs` — new module with:
  - `validate_discovery()` — issuer mismatch, trusted URL (HTTPS + loopback), PKCE S256, signing alg advertisement
  - `verify_id_token()` — RS256/ES256 JWKS signature verification, issuer, audience, nonce, iat/exp/nbf
  - `fetch_jwks()` — JWKS fetch with timeout
- Browser login (`server.rs`): nonce generation, discovery validation, ID token verified before `persist_tokens_async`
- Device code login (`device_code_auth.rs`): discovery validation, ID token verified, workspace check before persist
- All 5 OIDC unit tests pass (`oidc::tests::*`)

## Phase 2: Device auth / auth status — ✅ Done

- RFC 8628 compliance: `deserialize_interval` accepts both string and numeric `interval`; `authorization_pending`, `slow_down`, `expired_token`, `access_denied` handled
- Workspace restriction (`ensure_workspace_allowed`) wired into both browser and device flows
- Auth storage: CLI `from_auth_storage`, TUI `enforce_login_restrictions`, app-server `AuthManager` all use unified `load_auth` which supports `SAKRYLLE_CLI_HOME` and all `AuthCredentialsStoreMode` variants
- Random port strategy (port 0) replaces old fallback port logic
- Brand: `oauth_callback_error_message` outputs "Sakrylle is not enabled for your workspace"

## Phase 3: Sakrylle Responses API contract tests — ❌ Not started

- Need `/v1/responses` contract tests against mock or staging Sakrylle API

## Phase 4: Brand cleanup — ⚠️ Partial

- ✅ Error pages: `sakrylle_error.html`, `sakrylle_success.html`
- ✅ Entitlement error message: "Sakrylle is not enabled"
- ✅ Login server callback: Sakrylle-branded
- ⚠️ Device code prompt (`print_device_code_prompt`): still says "Welcome to Codex", "OpenAI's command-line coding agent", "sign in with ChatGPT"
- ⚠️ CLI help text / `bin_name`: not verified
- ⚠️ README/docs: not reviewed for Codex residue

## Phase 5: oidc-docs updates — ⚠️ In progress (this file)

- ✅ `implementation-status.md` updated
- ✅ `local-integration.md` — id_token validation gaps marked as resolved
- ✅ `troubleshooting.md` — reviewed, no new failure modes to document (revoke test failures are mock-only)

## Test status (2026-06-06)

| Suite | Pass | Fail | Notes |
|-------|------|------|-------|
| OIDC unit tests | 5 | 0 | |
| Other login unit tests | 84 | 0 | |
| login_server_e2e | 8 | 0 | |
| device_code_login | 6 | 0 | |
| logout/revoke | 2 | 3 | Pre-existing: revoke mock returns non-JSON, unrelated to OIDC changes |

## Suggested verification

- Verify `sakrylle` uses `~/.sakrylle-cli` rather than `~/.codex` by default.
- Run OAuth login against Sakrylle issuer in a non-production test profile.
- Confirm access tokens work for `/v1/responses` and usage is billed by Sakrylle API.
- Check logout/revoke clears local credentials and does not affect upstream Codex config.

