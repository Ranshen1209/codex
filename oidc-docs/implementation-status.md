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

⚠️ **全量深度去品牌 —— 独立后续专项（进行中，2026-06-10 第二轮）**：

第二轮（deep-rebrand 分支 `sakrylle/deep-rebrand`）产出：
- ✅ **权威残留清单**：`scripts/rebrand_audit.py` + `scripts/rebrand_audit.json` + `scripts/rebrand_audit_report.md`。分桶（排除测试/注释/协议值/内部路由后）：A(cosmetic)=328、B(归属/安装 URL)=13、C(功能端点/Apps)=17、SKIP=2530，actionable_files=147。
- ✅ **可达性核实推翻了「死代码」假设**：多处桶 C 实为 **live**——Apps UI（`app_link_view.rs` 仍接入 app 事件循环）、`CYBER_VERIFY_URL`（session 回退错误消息在用）、Remote Control 主机白名单（live CLI 子命令）、OTEL statsig 端点（在用）、announcement-tip 远程拉取、daemon 自更新。按安全门控 **停下回报，不静默删坏**。
- ✅ **已执行的安全去品牌**（display-only，已 `cargo build` 三 crate 通过 + `cargo fmt`）：`onboarding/auth.rs` 移除 `chatgpt.com/#settings` 链接；`core/session_rollout_init_error.rs` 4 条错误文案 Codex→Sakrylle；`cli/state_db_recovery.rs` 全部用户可见 `eprintln!` 诊断 + `codex doctor`→`sakrylle doctor`。
- ✅ **第二轮：按操作者决策移除 4 个 live 上游功能**（均已验证：相关 crate `cargo build`/`--tests` 通过，定向测试通过）：
  - **telemetry**：删除 `otel/config.rs` 的 `ab.chatgpt.com/otlp` 端点 + statsig key；`Statsig` exporter 恒解析为「不导出」（保留 enum 变体以兼容 provider/默认值；用户自配 OTLP 不受影响）。
  - **公告 + "Codex App" 推广**：删除 `tooltips.rs` 的 `ANNOUNCEMENT_TIP_URL` 远程拉取整段 announcement 子模块 + `prewarm()`、`APP_TOOLTIP`/`OTHER_TOOLTIP` 推广、`tooltips.txt` 的 codex-app 行；保留通用 tooltip + Fast 提示。
  - **反馈（Option A：仅 UI）**：删除 `feedback_view.rs` + 12 快照、`/feedback` slash 命令、`FeedbackCategory`/4 个 AppEvent、ChatWidget UI 方法、`FeedbackAudience`、提交链路。**保留**（load-bearing）`codex-feedback` 遥测/日志骨干、app-server `feedback/upload` v2 RPC（避免改 wire/schema，本环境无 `just`）、`config.feedback_enabled`。CLI 不再向 github.com/openai/codex 提交。
  - **应用内更新 + daemon 自更新**：删除 `update_action/update_prompt/updates/update_versions/npm_registry` 模块、`UpdateAvailableHistoryCell`、`cli/doctor/updates`、`app-server-daemon/update_loop`、`Update`/`PidUpdateLoop` 子命令、daemon 自更新接线、`lib.rs` 的 `chatgpt.com/codex/install.sh` 安装提示。`check_for_update_on_startup` 配置字段保留（读而不用，删除需 schema 重生成）；`app-server-daemon/Cargo.toml` 残留未用的 `reqwest`/`sha2`（删除需 bazel-lock 刷新）。
- ⏳ **交回操作者**：见 `docs/superpowers/2026-06-10-sakrylle-deep-rebrand-handoff.md` —— 含 (1) 仍待决策的 live 项（Apps UI、cyber、remote-control、usage-limit 文案、app-server `feedback/upload` RPC——需真实 Sakrylle URL/host 或改 v2 schema，不可臆造)；(2) must-not-change 清单（HTTP headers、provider id、env 键、issuer、内部路由、MCP 工具名）；(3) 剩余桶 A 分类（平台标识需跨平台构建核验、doctor 诊断串与内联测试耦合、模型面 prompt 文本属行为决策）；(4) 5 项真实环境/审批/多平台/发布手动验收。
- `update_action.rs` 的 `@openai/codex` 是**功能性**更新命令（非 cosmetic），应改为已确立的 `@sakrylle/cli` 或停用更新功能——交产品决策，未盲改。
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

