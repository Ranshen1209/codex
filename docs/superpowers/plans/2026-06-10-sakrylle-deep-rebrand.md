# Sakrylle Deep-Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove user-visible OpenAI/Codex/ChatGPT branding and dead upstream links from Sakrylle CLI, without breaking protocol values, config loading, or live functionality.

**Architecture:** Script-generated authoritative residue list (`scripts/rebrand_audit.py` → `scripts/rebrand_audit.json`) classifies every brand occurrence into buckets A (cosmetic), B (attribution/install URLs → remove), C (functional endpoints/Apps UI), or SKIP (tests, comments, protocol values, internal routing). Execute the safe/decided subset with per-crate verification; STOP-and-report any occurrence that reachability analysis shows is **live functional code** (per spec §3 桶C safety gate). Never invent a Sakrylle URL/credential.

**Tech Stack:** Rust (cargo, no `just` available → `cargo` directly), insta snapshots, ripgrep, Python audit script.

**Branch:** `sakrylle/deep-rebrand` (off `sakrylle/finishing-oidc-branding`).

---

## Reachability findings (gate inputs — verified 2026-06-10)

| Item | Status | Disposition |
|---|---|---|
| `codex-mcp host_owned_codex_apps_enabled` | hardcoded `false` | Apps MCP client dead |
| `tui/bottom_pane/app_link_view.rs` (Apps UI) | **wired into app event loop** (`app.rs`, `chatwidget.rs`, `thread_routing.rs` via `from_url_app_server_request`) | **LIVE-reachable → REPORT, do not bulk-remove** |
| `core/session/mod.rs CYBER_VERIFY_URL` | **used in live fallback error message** | **LIVE → REPORT** |
| `remote_control/protocol.rs` chatgpt.com allowlist | **live CLI subcommand** (`RemoteControl`, `--remote-control`) | **LIVE → REPORT** |
| `otel/config.rs STATSIG_OTLP_HTTP_ENDPOINT` | **used to build OTEL exporter** | **LIVE → REPORT (needs Sakrylle endpoint)** |
| `connectors/lib.rs connector_install_url` | builds chatgpt.com/apps URL | Apps-adjacent → REPORT |
| `tui/tooltips.rs ANNOUNCEMENT_TIP_URL` | live remote fetch (raw.githubusercontent openai/codex) | REPORT (needs Sakrylle URL) |
| `onboarding/auth.rs` `chatgpt.com/device`,`/backend-api` (lines 1049–1244) | **test-only** (`#[cfg(test)]`) | no action |
| `onboarding/auth.rs:576` `chatgpt.com/#settings` | live display hyperlink | **SAFE remove link** |

## Must-NOT-change inventory (protocol/infra — excluded from all buckets)

- HTTP headers: `OpenAI-Beta`, `X-OpenAI-Fedramp`, `X-OpenAI-Product-Sku`, `OpenAI-Organization`, `OpenAI-Project`.
- Provider id/display: `"openai"`, `OPENAI_PROVIDER_NAME = "OpenAI"`.
- Env/config keys: `CODEX_HOME`, `CODEX_API_KEY`, `OPENAI_API_KEY`, `CODEX_INTERNAL_ORIGINATOR_OVERRIDE`, etc.
- agent-identity JWT issuer constants (`chatgpt.com/codex-backend/...`, `backend-api`).
- Internal routing host-checks: `base_url.starts_with("https://chatgpt.com")`, `codex-client/chatgpt_hosts.rs`.
- Internal symbols: `SignInState::ChatGpt`, `uses_codex_backend`, `find_codex_home`, module/fn/type names.
- Upstream FS path components in config loaders (`Path::new("OpenAI").join("Codex")`) — verify before any touch.
- MCP tool name `codex` in `mcp-server/codex_tool_config.rs` — protocol identifier for MCP clients.
- `/snapshots/`, `*_tests.rs`, `tests.rs`, `/tests/`, `Cargo.toml`, attribution comments.

---

## Task 1: Commit authoritative audit (§4.1 deliverable)

**Files:** Create `scripts/rebrand_audit.py`, `scripts/rebrand_audit.json`, `scripts/rebrand_audit_report.md`.

- [ ] Finalize `scripts/rebrand_audit.py` (test-block + comment + protocol-value exclusions).
- [ ] Run `python3 scripts/rebrand_audit.py`; confirm summary (A≈328, B≈13, C≈17, actionable_files≈147).
- [ ] Generate `scripts/rebrand_audit_report.md` (per-bucket file:line + disposition).
- [ ] Commit: `chore(rebrand): add authoritative brand-residue audit`.

## Task 2: Bucket B — remove upstream repo/install/feedback URLs

**Files (Modify):** `tui/src/update_prompt.rs`, `tui/src/update_action.rs`, `tui/src/history_cell/notices.rs`, `tui/src/bottom_pane/feedback_view.rs`, `cli/src/doctor.rs`, `cli/src/doctor/updates.rs`.

Decision (spec §6.1/§6.2): **remove** links + install commands; fix prose so no dangling sentence/empty notice. Do NOT invent Sakrylle URLs.

- [ ] Per file: read each match in context, remove the link/command, repair surrounding text + any now-dead branch/const/import.
- [ ] `cargo build -p codex-tui -p codex-cli`.
- [ ] `cargo test -p codex-tui -p codex-cli`; `cargo insta pending-snapshots -p codex-tui`; review diffs are brand-only; `cargo insta accept -p codex-tui`.
- [ ] `cargo fmt`; commit `brand: remove upstream repo/install/feedback URLs (bucket B)`.

## Task 3: Bucket C safe — display-only debrand

**Files (Modify):** `tui/src/tooltips.rs` (Codex App promo strings only — NOT `ANNOUNCEMENT_TIP_URL`), `tui/src/onboarding/auth.rs:576`, `protocol/src/error.rs:125,517,533,538`.

- [ ] tooltips: remove/neutralize `APP_TOOLTIP`/`OTHER_TOOLTIP` "Codex App" promos; debrand `OTHER_TOOLTIP_NON_MAC`, `FREE_GO_TOOLTIP`. Keep `cfg!(macos/windows)` arms balanced.
- [ ] onboarding/auth.rs:576: remove `chatgpt.com/#settings` hyperlink, keep block coherent.
- [ ] protocol/error.rs: debrand ChatGPT-plan upsell messages — remove chatgpt.com upgrade/usage URLs, replace "Codex"/"ChatGPT plan" wording with neutral Sakrylle phrasing (no invented URL).
- [ ] `cargo build/test -p codex-tui -p codex-protocol`; insta review/accept.
- [ ] `cargo fmt`; commit `brand: debrand Codex App tooltips, onboarding link, usage-limit messages (bucket C safe)`.

## Task 4: Bucket A safe cosmetic slice

**Scope:** user-facing cosmetic `Codex`→`Sakrylle` in clearly user-visible, low-risk crates only: `cli/src/doctor*`, `core/src/session_rollout_init_error.rs`, `exec/`, `cli/src/state_db_recovery.rs`, `cli/src/main.rs`. **EXCLUDE** `cli/src/desktop_app/*` (platform ids, cfg-gated), `mcp-server/codex_tool_config.rs` (tool name), `core/src/guardian/*`, `core/src/realtime_*`, `memories*/prompts*` (model-facing), `config/src/loader/*` (paths).

- [ ] Per target file: review each bucket-A match; replace only genuine user-visible brand text. Skip identifiers/paths/headers.
- [ ] `cargo build`/`cargo test` per touched crate.
- [ ] `cargo fmt`; commit `brand: rebrand user-facing CLI cosmetic strings (bucket A safe slice)`.

## Task 5: Cross-platform cfg verification

- [ ] After Task 3/4, grep residual brand refs in any touched `#[cfg]` arms.
- [ ] If `x86_64-pc-windows-msvc` / linux targets installed: `cargo check --target ...` for touched crates; else manual per-`#[cfg]` audit and note in handoff. (Memory: `cfg-gated-cross-platform-verify`.)

## Task 6: Docs + operator handoff

**Files:** `oidc-docs/implementation-status.md`, create `oidc-docs/deep-rebrand-handoff.md`.

- [ ] Update Phase 4: list deep-rebrand completed (B, C-safe, A-slice) vs deferred.
- [ ] Handoff doc: (a) 5 real-env/approval/multiplatform/publish items from finishing spec §7; (b) STOP-report live endpoints (Apps UI, cyber, remote-control, otel, connectors, announcement/update URLs) with recommended action + why not auto-done; (c) must-not-change inventory; (d) risky bucket-A categories (platform ids, MCP tool name, model-facing prompts).
- [ ] Commit `docs(rebrand): sync status + operator handoff checklist`.

## Definition of Done (this run)

- Audit artifacts committed; residue list trustworthy.
- Bucket B cleared; bucket C-safe done; bucket A safe slice done — all verified (build/test/snapshots).
- Every live/ambiguous endpoint either left intact + reported, or handled with verification. No invented URLs/credentials/results.
- Docs synced; operator handoff precise.
- Remaining bucket A (risky/long-tail) explicitly enumerated as deferred, not silently dropped.
