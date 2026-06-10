---
title: Sakrylle CLI 收尾设计（OIDC 验证 + 品牌化）
date: 2026-06-10
status: approved-design
scope: product (Sakrylle CLI fork)
---

# Sakrylle CLI 收尾设计 —— OIDC 验证收尾 + 品牌化收尾

## 1. 背景与目标

Sakrylle CLI 是 OpenAI Codex CLI 的 fork。经代码核验（2026-06-10），OIDC 接入与品牌化改造主体已落地：

- **OIDC（~90%）**：严格 id_token 验签（`login/src/oidc.rs`，RS256+ES256，全套 claims）、一键 loopback PKCE 登录（`server.rs`，issuer 已切 `https://sub.sakrylle.com`、随机端口）、Device Flow（`device_code_auth.rs`，端点走 discovery）、refresh/logout/revoke（`auth/manager.rs`、`auth/revoke.rs`）、凭据隔离落盘 0600（`auth/storage.rs`）均已实现，5 项 OIDC 单测通过。
- **品牌化（~85%）**：`bin_name=sakrylle` + `skl` 别名、`SAKRYLLE_CLI_HOME` 配置隔离、Sakrylle provider（`api.sakrylle.com/v1`、`requires_openai_auth=false`）、`originator=sakrylle_cli_rs`、npm 包名 `@sakrylle/cli` 已改造。

**本设计目标**：把剩余收尾项做成可实现、可验证的工程计划，达成「完全实现 OIDC 接入 + Sakrylle 品牌化」。

**结构**：方案 A —— 双轨并行（两轨文件零重叠，各自独立 PR、独立 CI 绿灯交付），末尾附手动验收清单。

```
Sakrylle CLI 收尾
├── 轨道 1｜品牌化收尾   (PR-1, 低风险, 可先合)
│     ├─ 1a 用户可见文案 ×4
│     ├─ 1b npm 平台子包映射 + 命名锁定
│     └─ 1c oidc-docs 文档同步
├── 轨道 2｜OIDC 验证收尾 (PR-2, 在现有 wiremock 脚手架扩展)
│     ├─ 2a 修复 revoke 3 个失败单测
│     ├─ 2b /v1/responses mock 契约测试
│     └─ 2c 登录流端到端 mock 覆盖补强
└── 附录｜手动验收清单     (真实环境/生产审批, 不进 CI)
```

## 2. 范围

**纳入**：
- 4 处用户可见上游文案改写
- npm 平台子包映射修正 + 命名锁定
- oidc-docs 本地文档同步
- 修复遗留 revoke 单测失败
- `/v1/responses` mock 契约测试
- 登录流端到端 mock 覆盖补强

**不做（YAGNI）**：
- 不改 ChatGPT 后端依赖功能本身（仅改其用户可见文案）
- 不实现 Client Credentials / Hybrid Flow
- 不碰 sub2api 服务端（OIDC provider 基座已 100% 就绪，非本仓职责）
- 不改任何非用户可见的内部符号（如 `ChatGptXxx` 枚举/模块名）

## 3. 轨道 1 — 品牌化收尾（PR-1）

### 3.1 用户可见文案 ×4（仅改字符串，不动逻辑）

| 文件:行 | 现状 | 目标 |
|---|---|---|
| `codex-rs/login/src/device_code_auth.rs:204-205` | `Welcome to Codex [v…]` / `OpenAI's command-line coding agent` / `sign in with ChatGPT` | `Welcome to Sakrylle [v…]` / `Sakrylle CLI` / `sign in with Sakrylle`（保留 device code 步骤说明结构） |
| `codex-rs/tui/src/tooltips.rs:462` | `Welcome to Codex! Check out the new onboarding flow.` | `Welcome to Sakrylle! Check out the new onboarding flow.` |
| `codex-rs/tui/src/onboarding/auth.rs:445` | `Sign in with ChatGPT` | `Sign in with Sakrylle` |
| `codex-rs/tui/src/onboarding/auth.rs:471` | `API key login is disabled by this workspace. Sign in with ChatGPT to continue.` | `…Sign in with Sakrylle to continue.` |
| `codex-rs/cloud-tasks/src/lib.rs:80,92` | `Please run 'codex login' to sign in with ChatGPT, then re-run 'codex cloud'.` | `Please run 'sakrylle login' to sign in with Sakrylle, then re-run 'sakrylle cloud'.`（**两改**：品牌 + 命令名 `codex`→`sakrylle`） |

附带：若改动范围内发现金额展示用 `$`，统一为 `￥`（项目货币政策，仅展示不转换）。

### 3.2 npm 平台子包映射 + 命名锁定

- 文件：`codex-cli/bin/codex.js:16-23`
- 现 6 条映射全指向 `@openai/codex-<platform>`，会导致 `npx @sakrylle/cli` 拉到 OpenAI 子包、spawn 不到 Sakrylle 二进制。
- **锁定命名方案**：`@sakrylle/cli-<platform>`，镜像上游结构：

  | target triple | 子包名 |
  |---|---|
  | x86_64-unknown-linux-musl | `@sakrylle/cli-linux-x64` |
  | aarch64-unknown-linux-musl | `@sakrylle/cli-linux-arm64` |
  | x86_64-apple-darwin | `@sakrylle/cli-darwin-x64` |
  | aarch64-apple-darwin | `@sakrylle/cli-darwin-arm64` |
  | x86_64-pc-windows-msvc | `@sakrylle/cli-win32-x64` |
  | aarch64-pc-windows-msvc | `@sakrylle/cli-win32-arm64` |

- 本 PR 仅改映射表 + 锁定命名；实际把子包发布到 registry 属生产动作，列入附录手动审批项。

### 3.3 oidc-docs 文档同步

- 文件：`oidc-docs/implementation-status.md`
- Phase 3 由 ❌「未开始」→ ✅「代码完成，待真实环境验证」。
- Phase 4 勾掉本次已改的文案残留项（device code prompt 等）。
- 更新「Test status」表（revoke 单测修复后由 2 通过 → 5 通过）。

## 4. 轨道 2 — OIDC 验证收尾（PR-2）

全部基于现有 `wiremock` 脚手架（`oidc.rs` / `revoke.rs` 已在用）与 `login/tests/suite/` 既有套件扩展，零真实凭据、可进 CI。

### 4.1 修复 revoke 3 个失败单测

- 文件：`codex-rs/login/tests/suite/logout.rs`、`codex-rs/login/src/auth/revoke.rs`
- **先确认根因**（系统化调试）：文档假设为 mock 返回非 JSON body，而 revoke 解析时强求 JSON。
- 倾向修复方向（根因确认后再定补丁，不预设）：按 RFC 7009，吊销端点成功返回 **200 + 空/任意 body**，因此让 `revoke.rs` 把 200 视为成功、不强解析 JSON（既修测试也是真实正确性增强）；若根因实为 mock 配置不符规范，则修 mock。
- 验收：`just test -p codex-login` 全绿，logout/revoke 由 2 通过 → 5 通过。

### 4.2 `/v1/responses` mock 契约测试

- 新文件：`codex-rs/login/tests/suite/responses_contract.rs`（或就近放入 core/suite 集成测试，按既有约定择优）
- 用 wiremock 起 mock Sakrylle API，断言 CLI 发往 `/v1/responses` 的请求满足契约：
  - 路径/形态符合 `wire_api=responses`
  - `Authorization: Bearer sk_oauth_…`
  - originator / User-Agent = `sakrylle_cli_rs`
  - base_url 取自 Sakrylle provider（`api.sakrylle.com/v1`，可被 `SAKRYLLE_API_BASE_URL` 覆盖）
- 覆盖错误分支：401 鉴权失败、workspace 不符时客户端处理。
- 落地文档 Phase 3（当前 ❌）缺失契约测试在可自动化范围内的部分。

### 4.3 登录流端到端 mock 覆盖补强

- 文件：扩展 `codex-rs/login/tests/suite/login_server_e2e.rs`、`device_code_login.rs`
- **先盘点**现有套件已覆盖哪些路径，只补缺口，不重复造测试。
- 正路径：mock IdP 发 RS256 **与** ES256 两种 id_token，断言两者都验签通过（对应 `ALLOWED_ID_TOKEN_ALGS={RS256,ES256}`）。
- 负路径：`alg=none` / 未知 alg 被拒、nonce 不符被拒、refresh rotation 后旧 token 被拒。

## 5. 测试策略

- **轨道 1**：`just fmt`；TUI 文案改动若触发 insta 快照差异，`cargo insta pending-snapshots -p codex-tui` 检查后 `cargo insta accept -p codex-tui`。
- **轨道 2**：`just test -p codex-login`（nextest）；遵循项目测试规范（集成测试优先、AAA、描述性命名）。
- 两轨各自 CI 绿灯方可交付。

## 6. 交付与回滚

- **PR-1（品牌化）**：低风险，可先合，不阻塞 PR-2。
- **PR-2（OIDC 验证）**：独立合。
- 各 PR 对应一个子任务组。
- **回滚**：纯客户端改动，回滚 = 还原文案/映射/测试文件；不触生产 sub2api，access_token 路径零回归。

## 7. 附录 — 手动验收清单（不进 CI，需外部资源/审批）

1. **真实端到端**：非生产 profile 下 `sakrylle login`（一键）+ `--device` + refresh + `logout`，全程零复制粘贴。
2. **真实计费**：`SAKRYLLE_API_KEY=<Claude-group key> sakrylle exec "…"` → sub2api `usage_logs` 落 `total_cost>0`。
3. **生产 `oauth_clients`（需审批）**：核对 `sakrylle-cli` 的 `redirect_uris` 含通配 loopback（`http://127.0.0.1:*/callback` + `http://localhost`），缺则补 seed。
4. **三平台实测**：macOS / Linux / Windows 浏览器拉起 + loopback 回调；SSH 远程 `--device`。
5. **npm 发布（需审批）**：`@sakrylle/cli-<platform>` 6 个子包发布到 registry，`npx @sakrylle/cli login` 跑通。

## 8. 验收标准（Definition of Done）

- 全仓搜不到用户可见的 `Welcome to Codex` / `OpenAI's command-line coding agent` / `sign in with ChatGPT`（内部符号除外）。
- `codex-cli/bin/codex.js` 子包映射全部为 `@sakrylle/cli-*`。
- `oidc-docs/implementation-status.md` 与代码现状一致。
- `just test -p codex-login` 全绿（含 revoke、新增契约测试、补强的登录流测试）。
- 附录手动清单已记录、明确标注「需审批 / 需真实环境」，交由后续执行。
