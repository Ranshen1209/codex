---
title: Sakrylle CLI 全量去品牌（深度 rebrand）设计
date: 2026-06-10
status: design (§6 决策已定 2026-06-10；保留可达性核实 + 真实环境验证为执行期要求)
scope: product (Sakrylle CLI fork) — 独立后续专项
supersedes_note: 承接 2026-06-10-sakrylle-cli-finishing-design.md 中标记为「deferred」的全量去品牌
---

# Sakrylle CLI 全量去品牌（深度 rebrand）设计

## 1. 背景

Sakrylle CLI 是 OpenAI Codex CLI 的 fork。收尾工作（见 `2026-06-10-sakrylle-cli-finishing-design.md`）已完成**目标性**的高价值品牌改动（设备码提示、TUI onboarding/状态/登录文案、移除上游 install/usage/cyber 链接、npm 子包映射、cloud-tasks 等）。

但 2026-06-10 的审计显示残留规模远超收尾范围：`codex-rs` 非测试源码中含 `Codex`/`OpenAI`/`ChatGPT` 的用户可见字符串字面量 **约 660 处，分布在约 185 个文件**（数字为粗估，含部分内联测试代码）。本 spec 专门处理这部分剩余的全量去品牌。

**为何独立成专项**：逐字符串人工改动 660 处不现实且易错；且残留中混有**功能性端点**与**基础设施 URL**，盲改会引入 bug 或指向不存在的页面。需要脚本化 + 分层人工核对 + 若干产品决策。

## 2. 目标

让 Sakrylle CLI 的**用户可见**界面、文档 URL、升级/反馈引导彻底不出现上游 OpenAI/Codex/ChatGPT 品牌或失效的上游链接，同时**不破坏**任何功能性端点与跨平台构建。

**非目标 / 明确不做**：
- 不改 `agent-identity` 的 JWT issuer 常量（`chatgpt.com/codex-backend/...`）等**协议/基础设施值**——改动会破坏验签。
- 不改为上游兼容**有意保留**的 env 变量名 / 配置键（`CODEX_HOME`、`CODEX_API_KEY`、`OPENAI_API_KEY`、`CODEX_INTERNAL_ORIGINATOR_OVERRIDE` 等）。
- 不改内部符号（枚举变体、模块名、函数/变量名，如 `SignInState::ChatGpt`、`uses_codex_backend`、`find_codex_home`）——非用户可见，改动风险大、收益低。
- 不重复收尾分支已完成的目标项。
- 不在本专项内做与品牌无关的功能重构。（注：ChatGPT Apps 生态 UI 的**移除**已纳入本专项——见 §3 桶 C、§6 决策 4——因其属品牌面 + `codex_apps` 禁用后的死代码，而非独立功能重构。）

## 3. 残留分类（三桶 + 处理原则）

基于审计，所有残留归入三桶，处理方式不同：

### 桶 A — 纯 cosmetic 文案（占大多数）
用户可见但无外部依赖的品牌文本（标题、提示、帮助、错误消息等）。
- **处理**：脚本化批量替换 `Codex`→`Sakrylle`、`OpenAI Codex`→`Sakrylle` 等，逐文件 diff 人工确认，避免误伤内部符号/保留变量。

### 桶 B — 基础设施 / 归属 URL
指向上游仓库/站点的链接，多为面向用户展示：
- `github.com/openai/codex` 的 release notes（`tui/src/update_prompt.rs`）、feedback issue（`tui/src/bottom_pane/feedback_view.rs`）、安装提示（`tui/src/history_cell/notices.rs`）；
- TUI 内升级命令仍写 `npm install -g @openai/codex` / `brew upgrade --cask codex`（`tui/src/update_action.rs`），与已改的 `codex.js`（`@sakrylle/cli-*`）不一致；
- 收尾分支移除自更新 chatgpt.com 命令后，fallback notice 仍指向 `github.com/openai/codex`。
- **处理（决策 2026-06-10）：一律移除**这些链接 / 升级命令 / 反馈入口，**不**替换为臆造的 Sakrylle URL。移除后必须修周边文案使句子通顺（不留断句或空 notice）。日后若 Sakrylle 确立了真实仓库/安装渠道/反馈渠道，再作为新增项单独接回。

### 桶 C — 功能性端点（高风险）
真实端点字符串，改错会出 bug：
- `chatgpt.com/backend-api/` 基址（`tui/src/lib.rs`、`tui/src/session_archive_commands.rs`、`onboarding/auth.rs`）；
- OAuth/`chatgpt.com/device`、`chatgpt.com/#settings`（`onboarding/auth.rs`，多处疑似 legacy ChatGPT 登录路径或内联测试代码）；
- ChatGPT "Apps" 生态 UI（`bottom_pane/app_link_view.rs`、`chatwidget/plugins.rs`、`codex-mcp/auth_elicitation.rs`、`chatgpt.com` host 校验）——`codex_apps` MCP 客户端在 Sakrylle 已禁用（`codex-mcp/src/mcp/mod.rs`，`apps_enabled: false`），故这些 UI 疑似不可达死代码。
- **处理（决策 2026-06-10）：一律移除**（ChatGPT Apps UI 整块移除；`chatgpt.com/backend-api`·`/device`·`/#settings` 移除）。但**移除 = 移除使用该端点的代码路径**，不是把 URL 字符串清空留下断掉的调用。**执行期安全前提（强制）**：删每一项前先核实可达性（grep 调用链，判断是否已被 Sakrylle OIDC（`sub.sakrylle.com`）路径取代、或因 `codex_apps` 禁用而死代码）；**若某处实为 live 且承载 Sakrylle 仍需的功能，停止并回报**，不得静默删坏。死代码/legacy 路径则整段移除（含相关 import/const/分支/测试）。

## 4. 方法（分层执行）

1. **生成权威清单**：脚本扫描 `codex-rs/` + `codex-cli/`，输出每处残留的 `file:line + 字符串 + 所属桶`（复用收尾期审计逻辑）。排除 `/snapshots/`、`*_tests.rs`、`/tests/`、`Cargo.toml`、代码注释中的上游归属、保留 env/config 键。
2. **桶 A（脚本化）**：受控批量替换 + 逐文件人工 diff 复核；先在小批 crate 验证替换规则不误伤。
3. **桶 B（人工，依赖 §6 决策）**：按确定的真实 Sakrylle URL 替换，或移除链接 + 修周边文案使句子通顺（避免移除后留下断句）。
4. **桶 C（先核实再动）**：对每个功能端点先判定可达性（grep 调用链 + 判断是否 legacy/死代码）；可达的须指向正确 Sakrylle 端点并测试；死代码可删或骿名，单独 review。
5. **快照与跨平台**：UI 文案改动会触发 `codex-tui` insta 快照——逐一确认仅品牌差异后 accept。**凡涉及 `#[cfg(平台)]` 门控代码的改动，必须按目标平台核验**（见项目记忆 `cfg-gated-cross-platform-verify`：宿主全绿不充分）。

## 5. 验证策略

- 分层提交（桶 A / 桶 B / 桶 C 各自独立 PR 或 commit 组），便于 review 与回滚。
- 每批：`cargo build` 相关 crate + 受影响 crate 的 `cargo test` + `cargo insta` 快照确认。
- 跨平台：对触及平台门控代码的改动，`cargo check --target x86_64-pc-windows-msvc`（及 linux 目标）若工具链可用；否则人工核查每个 `#[cfg]` 块。
- 完成判据：脚本清单中桶 A/B 清零（桶 B 中确属「移除」的已移除）；桶 C 每项有明确处置（改/删/留 + 理由）；全量构建不因本专项新增失败；现有功能（登录、responses、刷新、登出）回归通过。
- **同时清理收尾期已知的既有遗留**（顺带，非新增）：`codex-core` 全量 test 的 `supports_image_generation` 编译中断、`codex-tui` 因更早 rebrand 提交而过时的快照、`server.rs:271-273` redundant clone——这些虽非本专项引入，但属同类品牌/收尾债，建议在本专项一并收口或单列跟踪。

## 6. 决策记录与执行期要求

**已决策（2026-06-10，用户拍板）—— 一律「移除」，不替换为臆造值：**

1. **真实 Sakrylle 仓库 URL** → **移除**。release notes / feedback issue / 安装说明中的 `github.com/openai/codex` 链接全部移除（不接 Sakrylle 仓库链接）。
2. **Sakrylle 安装/升级渠道** → **移除**。TUI 升级提示中的 `npm install -g @openai/codex` / `brew ... codex` / 安装脚本命令移除（不替换为 Sakrylle 命令）。
3. **Sakrylle 用量/设置页** → **移除**。`chatgpt.com/codex/settings/usage`、`chatgpt.com/#settings` 入口移除（保留本地用量数据展示，仅去掉外链）。
4. **ChatGPT Apps 生态 UI** → **整块移除**（`app_link_view.rs`、`chatwidget/plugins.rs`、`codex-mcp/auth_elicitation.rs` 相关；`codex_apps` 已禁用，属死代码）。
5. **`chatgpt.com/backend-api`·`/device`·`/#settings`** → **移除**（移除使用它们的代码路径，见 §3 桶 C 安全前提）。

> 日后 Sakrylle 若确立真实仓库 / 安装渠道 / 用量页，再作为**新增**项单独接回，不在本专项内臆造。

**执行期仍须满足（非未决，是约束）：**
- **可达性核实（强制，针对决策 4/5）**：删除任何功能端点 / Apps UI 前先 grep 调用链确认其为死代码/legacy；若发现实为 live 且承载 Sakrylle 仍需功能，**停止并回报**，不得静默删坏（见 §3 桶 C、§7 风险）。
- **真实环境验证**：涉及登录 / responses / 设备授权路径的移除，须在非生产 profile 实测无回归（可能需真实凭据，对齐收尾期手动验收清单）。
- **跨平台核验**：凡触及 `#[cfg(平台)]` 门控代码的移除，按目标平台核验（见项目记忆 `cfg-gated-cross-platform-verify`）。

## 7. 风险

- **功能回归**（桶 C 改错端点）——以「先核实可达性、可达项必测」缓解。
- **跨平台编译**（平台门控代码）——以目标平台 check / 人工 cfg 核查缓解（见记忆）。
- **快照噪声**——大量 UI 文案改动产生大批快照 diff，需耐心逐批确认仅品牌差异。
- **误伤内部符号 / 保留变量**——替换脚本须排除符号与保留 env/config 键，逐 diff 复核。
