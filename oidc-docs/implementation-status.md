---
title: Sakrylle CLI Implementation Status
status: local
scope: product-local
canonical_source: ../../sub2api/sakrylle-docs/10-platform-identity/current-state.md
last_verified: 2026-06-10
---

# Sakrylle CLI Implementation Status

Current status: **Phase 1–3 代码完成（OIDC 严格验证 + 设备授权 + 登录流自动化测试覆盖）。品牌化主体已改，但全量去品牌为独立后续专项（见 Phase 4）。真实环境验证与生产发布为手动收尾项。**

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

## Phase 3: Sakrylle Responses API contract tests — ✅ Done (mock 自动化部分)

- 已新增 `/v1/responses` mock 契约集成测试：`codex-rs/core/tests/sakrylle_responses_contract.rs` —— 断言出站请求命中 Responses 端点、携带 `Authorization: Bearer`、请求体含非空 `input` 数组（Responses wire API 形态）。
- 登录流自动化覆盖补强（`codex-rs/login/src/oidc.rs` 单测）：ES256 正向验签、HS256 算法降级拒绝（验证 `ALLOWED_ID_TOKEN_ALGS`）、nonce 不符拒绝。refresh rotation 拒绝已由 `tests/suite/auth_refresh.rs` 覆盖。
- ⚠️ 真实 staging/生产 Sakrylle API 的端到端契约（真实 token、真实计费）仍为手动验收项（见文末）。

## Phase 4: Brand cleanup — ⚠️ 目标项已改，全量去品牌为独立后续专项

已完成（本轮，均经测试/快照校验）：
- ✅ 设备码登录提示（`device_code_auth.rs`）：去 "Welcome to Codex / OpenAI's command-line coding agent / sign in with ChatGPT"。
- ✅ TUI onboarding / tooltip：`Sign in with Sakrylle`、`Welcome to Sakrylle`、登录成功/禁用提示、状态卡 auth 标签等用户可见文案。
- ✅ `cli/login.rs`、`cli/doctor.rs`、`app-server-daemon` 标题的品牌文案。
- ✅ 移除指向上游专有产品/链接的项：`update_action.rs` 自更新安装命令（chatgpt.com/codex/install.*）、`status/card.rs` 用量链接（chatgpt.com/codex/settings/usage）、`chatgpt.com/cyber` "Trusted Access for Cyber" 提示。
- ✅ npm 平台子包映射 `@openai/codex-*` → `@sakrylle/cli-*`（`codex-cli/bin/codex.js`）。
- ✅ `cloud-tasks` 未登录提示 + 命令名（`codex`→`sakrylle`）。

⚠️ **全量深度去品牌 —— 独立后续专项（deferred）**：
- 审计（2026-06-10）显示 `codex-rs` 非测试源码仍有 **约 660 处** 含 `Codex`/`OpenAI`/`ChatGPT` 的用户可见字符串，分布在 **约 185 个文件**。本轮只处理了高价值/明确项。
- 剩余项分三类，**不宜逐串手改**，建议作为专项用脚本化批量替换 + 人工核对：
  1. **纯 cosmetic 文案**（大多数）—— 可批量替换。
  2. **基础设施/归属 URL**：`github.com/openai/codex` 的 release-notes（`update_prompt.rs`）、feedback issue（`feedback_view.rs`）、安装提示（`history_cell/notices.rs`）等 —— 需真实 Sakrylle 仓库/域名才能替换（不可臆造）。
  3. **功能性端点**：`chatgpt.com/backend-api/` 基址、OAuth/`chatgpt.com/device` 设备授权、`chatgpt.com/#settings` —— 多为被 Sakrylle OIDC 路径取代的 legacy/测试代码（疑似死代码），改前须逐一确认是否仍可达。
- 已禁用的 `codex_apps` MCP 客户端相关的 ChatGPT "Apps" UI 字符串（`app_link_view.rs` / `chatwidget/plugins.rs` / `codex-mcp/auth_elicitation.rs`）属不可达死代码，归入此专项一并处理。
- `agent-identity` 的 JWT issuer 常量（`chatgpt.com/codex-backend/...`）是协议/基础设施值，**不应**作为文案改动（改动会破坏 agent-identity 验签）。

## Phase 5: oidc-docs updates — ✅ Done

- ✅ `implementation-status.md` updated
- ✅ `local-integration.md` — id_token validation gaps marked as resolved
- ✅ `troubleshooting.md` — reviewed, no new failure modes to document

## Test status (2026-06-10)

| Suite | Pass | Fail | Notes |
|-------|------|------|-------|
| OIDC unit tests (`oidc::tests`) | 8 | 0 | +3 新增：ES256 正向、HS256 拒绝、nonce 不符 |
| Other login unit tests | 85 | 0 | 含 `server::tests` revoke 持久化测试（已修） |
| login_server_e2e | 8 | 0 | |
| device_code_login | 6 | 0 | |
| logout/revoke | 5 | 0 | 修复：revoke 请求体断言由 JSON 改为 form-encoded（RFC 7009），覆盖 `logout.rs` ×2 + `server.rs` ×1 |
| `/v1/responses` contract (core) | 1 | 0 | 新增 `sakrylle_responses_contract.rs` |

> `cargo test -p codex-login` 全绿（93 lib + 35 integration）。`/v1/responses` 契约测试以独立 target 运行（`cargo test -p codex-core --test sakrylle_responses_contract`）；注意 `cargo build --tests -p codex-core` 存在与本次无关的**既有**编译问题（某测试构造 `ModelProviderInfo` 缺 fork 新增的 `supports_image_generation` 字段），不在本次范围。
> 既有未修项：`login/src/server.rs:271-273` 的 clippy `redundant clone`（fork OIDC 代码，预先存在）。

## Suggested verification

- Verify `sakrylle` uses `~/.sakrylle-cli` rather than `~/.codex` by default.
- Run OAuth login against Sakrylle issuer in a non-production test profile.
- Confirm access tokens work for `/v1/responses` and usage is billed by Sakrylle API.
- Check logout/revoke clears local credentials and does not affect upstream Codex config.

