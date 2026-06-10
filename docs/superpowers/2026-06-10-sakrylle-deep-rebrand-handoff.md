# Sakrylle Deep-Rebrand — Operator Handoff

**Date:** 2026-06-10
**Branch:** `sakrylle/deep-rebrand` (off `sakrylle/finishing-oidc-branding`)
**Status:** authoritative audit complete; safe display-only debrand executed; the remainder requires product decisions, real Sakrylle URLs/endpoints, real credentials, or multi-platform machines and is documented below. **No URLs/credentials/results were invented.**

Authoritative residue list: `scripts/rebrand_audit.py` → `scripts/rebrand_audit.json` + `scripts/rebrand_audit_report.md`. Re-run `python3 scripts/rebrand_audit.py` after any change to refresh.

Audit summary (after exclusions): **A(cosmetic)=328, B(attribution/install URLs)=13, C(functional/Apps)=17, SKIP=2530, actionable_files=147.**

---

## 1. What was changed in this run (safe, no product decision needed)

All edits are display-only user-facing cosmetic text; none touch protocol values, routing, or feature behavior.

| File | Change |
|---|---|
| `tui/src/onboarding/auth.rs:573-581` | Removed the `https://chatgpt.com/#settings` "training data preferences" hyperlink; replaced with plain `"Uses your plan's rate limits."` (block already said "Powered by your Sakrylle account"). |
| `core/src/session_rollout_init_error.rs` | 4 user-facing error hints `Codex`→`Sakrylle` ("Sakrylle cannot access session files…", "…different Sakrylle home", "…so Sakrylle can create sessions", "…directory Sakrylle can use…"). |
| `cli/src/state_db_recovery.rs` | All user-facing `eprintln!` diagnostics `Codex`→`Sakrylle`; `Run \`codex doctor\``→`Run \`sakrylle doctor\``. Internal `codex-repair-<ts>` temp-file suffix left unchanged. |

Plus pre-existing rustfmt-only changes in 4 `login/` files (cosmetic, from prior OIDC work).

**Suggested commit grouping** (commits not made — per CLAUDE.md, commit only on operator request):
1. `chore(rebrand): add authoritative brand-residue audit` — `scripts/rebrand_audit.*`, `docs/superpowers/specs/2026-06-10-sakrylle-cli-deep-rebrand-design.md`, `docs/superpowers/plans/2026-06-10-sakrylle-deep-rebrand.md`.
2. `brand: debrand safe user-facing strings (onboarding link, db/session diagnostics)` — the 3 source files above.
3. `style(login): apply rustfmt` — the 4 login files.

---

## 2. STOP-and-report: LIVE functional code

Reachability analysis (recorded in the plan doc) proved these are **live**, contradicting the spec's "dead code" assumption.

**Update (2026-06-10, round 2):** the operator approved **removing** the update / feedback / telemetry / announcement features. These have now been removed (✅ rows below). The remaining rows (Apps UI, Cyber, Remote Control) still need a product decision and/or a real Sakrylle value that must not be invented.

### Removed this round (verified — all touched crates compile, targeted tests pass)

| Feature | What was removed | Notes |
|---|---|---|
| **Telemetry** | `otel/src/config.rs`: deleted `STATSIG_OTLP_HTTP_ENDPOINT`/`STATSIG_API_KEY*` (the `ab.chatgpt.com/otlp` endpoint + key); `Statsig` exporter now always resolves to no export. | `Statsig` enum variant kept for `provider.rs`/windows-sandbox/config-default compatibility (no-op). User-configured OtlpHttp/OtlpGrpc unaffected. Test `statsig_metrics_exporter_resolves_to_no_export` passes. |
| **Announcement + "Codex App" promos** | `tui/src/tooltips.rs`: removed `ANNOUNCEMENT_TIP_URL` remote fetch + whole `announcement` submodule + `prewarm()` call (`lib.rs`); removed `APP_TOOLTIP`/`OTHER_TOOLTIP` chatgpt.com promos; removed `tooltips.txt` "codex app" line; dropped unused `IS_MACOS`/`IS_WINDOWS` split. | Generic tooltip pool + Fast-mode promo kept. Tooltip tests pass. |
| **Feedback (Option A — UI only)** | Deleted `tui/src/bottom_pane/feedback_view.rs` + 12 snapshots; removed `/feedback` slash command, `FeedbackCategory`, 4 feedback `AppEvent`s, ChatWidget UI methods, `FeedbackAudience` + `@openai.com` derivation, submit plumbing, `/feedback` tooltip + turn_runtime reference. | **Kept (load-bearing):** `codex-feedback` crate (tracing/Sentry/auth-tags backbone), the app-server `feedback/upload` v2 RPC (avoids wire/schema change — `just` schema-gen unavailable here), `config.feedback_enabled`. The CLI no longer submits to github.com/openai/codex. |
| **In-app update + daemon self-update** | Deleted `update_action.rs`, `update_prompt.rs`, `updates.rs`, `update_versions.rs`, `npm_registry.rs`, `cli/doctor/updates.rs`, `app-server-daemon/update_loop.rs`, `cli/tests/update.rs`, `wsl_paths.rs`; removed `UpdateAvailableHistoryCell`, the `Update`/`PidUpdateLoop` CLI subcommands, daemon updater wiring, and the `chatgpt.com/codex/install.sh` install hint in `app-server-daemon/lib.rs`. | `is_bootstrapped` redefined as "managed app-server backend running" (kept remote-control intact). `check_for_update_on_startup` config field kept read-but-unused (deleting needs `just write-config-schema`). Unused `reqwest`/`sha2` deps remain in `app-server-daemon/Cargo.toml` (removing needs `just bazel-lock-update`). `padded_emoji` now-unused (dead_code warning). |

### Still needs operator decision (NOT removed — live or contract-touching)

Each needs a product decision and/or a real Sakrylle value that must not be invented.

| Item | File(s) | Why live | Recommended action |
|---|---|---|---|
| **app-server `feedback/upload` RPC** | `app-server-protocol/src/protocol/v2/feedback.rs`, `app-server/src/request_processors/feedback_processor.rs` | Public v2 JSON-RPC method for IDE clients; removing changes the wire protocol + TS bindings + JSON schema (needs `just write-app-server-schema`, unavailable here). | If Sakrylle's IDE clients won't use it: remove the variant + types + processor and regenerate the schema in an environment with `just`. |
| **Apps UI** | `tui/src/bottom_pane/app_link_view.rs`, `codex-mcp/src/auth_elicitation.rs`, `connectors/src/lib.rs:427` | `app_link_view` is **wired into the app event loop** (`app.rs`, `chatwidget.rs`, `thread_routing.rs` via `from_url_app_server_request`), even though `host_owned_codex_apps_enabled()` returns `false`. | Confirm no app-server path emits app-link requests for Sakrylle, then remove the view + handlers as a dedicated change (touches app.rs orchestration). Not a string edit. |
| **Cyber trusted-access** | `core/src/session/mod.rs:425` (`CYBER_VERIFY_URL`) | **Used live** in a usage fallback error message referencing gpt-5.x model routing + chatgpt.com/cyber. | Product decision: does Sakrylle surface this fallback? If not, remove the branch; if yes, supply Sakrylle URL. |
| **Remote Control allowlist** | `app-server-transport/src/transport/remote_control/protocol.rs`, `enroll.rs` | **Live CLI subcommand** (`sakrylle ... --remote-control`, `RemoteControl`). The `chatgpt.com` host check gates where the phone/remote control connects. | Needs Sakrylle's remote-control host (real value) or feature disable. Do not remove the allowlist blindly. |
| **Usage-limit messages** | `protocol/src/error.rs:122,517,533,538` | Plan-based ("ChatGPT plan", Plus/Pro) upsell messages with chatgpt.com URLs, keyed on backend `PlanType`. | Decide whether Sakrylle surfaces plan-based usage limits; if yes, debrand wording + drop chatgpt.com URLs (keep local usage display per §6.3); if no, simplify to generic "usage limit reached". |

---

## 3. Must-NOT-change inventory (protocol/infra — verified, leave as-is)

These match the brand regex but are **protocol/infrastructure values**; changing them breaks compatibility, routing, or signature verification:

- **HTTP headers:** `OpenAI-Beta`, `X-OpenAI-Fedramp`, `X-OpenAI-Product-Sku`, `OpenAI-Organization`, `OpenAI-Project`.
- **Provider id/name:** `"openai"` provider id, `OPENAI_PROVIDER_NAME = "OpenAI"` (the upstream provider entry kept for compat alongside the Sakrylle provider).
- **Env/config keys:** `CODEX_HOME`, `CODEX_API_KEY`, `OPENAI_API_KEY`, `CODEX_INTERNAL_ORIGINATOR_OVERRIDE`, etc.
- **agent-identity JWT issuer** constants (`chatgpt.com/codex-backend/...`) — changing breaks agent-identity verification.
- **Internal host-routing checks:** `base_url.starts_with("https://chatgpt.com")` in `backend-client`, `cloud-tasks`, `codex-mcp`; `codex-client/src/chatgpt_hosts.rs` — upstream-compat logic like `uses_codex_backend`. For Sakrylle (`api.sakrylle.com`) these branches are simply not taken; not user-visible.
- **Internal symbols:** `SignInState::ChatGpt`, `uses_codex_backend`, `find_codex_home`, module/type/fn names.
- **Upstream FS path components** in config loaders (`Path::new("OpenAI").join("Codex")`) — used for upstream config-dir detection; verify intent before any touch.
- **MCP tool name `codex`** + tool descriptions in `mcp-server/src/codex_tool_config.rs` — protocol identifier exposed to MCP clients; renaming breaks MCP integrations.
- Tests (`*_tests.rs`, `tests.rs`, `/tests/`, inline `#[cfg(test)]`), comments (attribution retained), `Cargo.toml`, `/snapshots/`.

---

## 4. Deferred bucket-A (cosmetic, but needs care — not done this run)

The remaining ~315 bucket-A occurrences are user-visible cosmetic text but cluster into categories with non-trivial risk/coupling. Process each as its own reviewed change (see `scripts/rebrand_audit_report.md` for exact file:line):

- **`cli/src/desktop_app/mac.rs` (20), `windows.rs` (5):** macOS/Windows app-bundle identifiers/plist/app names — `#[cfg(target_os)]`-gated. **Requires cross-platform build verification** (`cargo check --target x86_64-pc-windows-msvc` / linux) per project memory `cfg-gated-cross-platform-verify`; host build does not cover them. Some are bundle IDs (may be protocol-ish).
- **`cli/src/doctor*.rs` (~40):** diagnostic output. Many strings ("ChatGPT auth", "OpenAI auth") are coupled to the `requires_openai_auth` config field and **asserted verbatim in extensive inline tests** — debranding requires updating those assertions in the same change.
- **`core/src/guardian/prompt.rs`, `realtime_prompt.rs`, `realtime_context.rs`, `memories*/prompts*`:** **model-facing prompt text**. Changing affects agent behavior — product/behavior decision, not pure cosmetics.
- **`mcp-server/codex_tool_config.rs` (13):** see must-not-change (tool name is protocol).
- **`config/src/loader/mod.rs`:** path/dir strings tied to config loading — verify before touch.
- **Remainder** (app-server-daemon, exec, protocol, model-provider, etc.): mostly safe cosmetic log/error text; low risk, mechanical, but spread across many crates each needing a build.

Recommended approach: drive from `scripts/rebrand_audit_report.md`, one crate-group per commit, `cargo build`/targeted tests per crate, `cargo insta` for `codex-tui`.

---

## 5. Manual acceptance items (real credentials / production approval / multi-platform / publish)

Carried from `2026-06-10-sakrylle-cli-finishing-design.md §7` — these cannot be done in an autonomous code environment and must not be faked:

1. **Real end-to-end:** non-prod profile `sakrylle login` (one-click) + `--device` + refresh + `logout`, fully copy-paste-free.
2. **Real billing smoke:** `SAKRYLLE_API_KEY=<Claude-group key> sakrylle exec "…"` → sub2api `usage_logs` records `total_cost>0`.
3. **Production `oauth_clients` (needs approval):** verify `sakrylle-cli` `redirect_uris` include wildcard loopback (`http://127.0.0.1:*/callback` + `http://localhost`); seed if missing.
4. **Three-platform test:** macOS / Linux / Windows browser launch + loopback callback; SSH-remote `--device` flow.
5. **npm publish (needs approval):** publish the 6 `@sakrylle/cli-<platform>` sub-packages to the registry; verify `npx @sakrylle/cli login`.

---

## 6. OIDC status

OIDC integration code is complete and tested (Phases 1–3 in `oidc-docs/implementation-status.md`); `cargo test -p codex-login` green. The safe rebrand edits in this run do **not** touch the OIDC login/device/refresh/revoke paths (`login/src/oidc.rs`, `server.rs`, `device_code_auth.rs`, `auth/*`). The removed `chatgpt.com/#settings` link and the `chatgpt.com/device`/`/backend-api` strings in `onboarding/auth.rs` are display/test-only and not part of the live Sakrylle OIDC flow.
