# Sakrylle 生态文档集

> 规划文档集（planning only）。本目录下所有文档均为设计与研究记录，**不含**直接改生产配置、破坏性迁移或删用户数据的操作指令。Sakrylle API（sub2api fork）与 Sakrylle Image 已上线生产；所有触及两者的操作步骤须经额外审批。

---

## 安全约束声明

**本文档集仅为规划文档。**

- Sakrylle API（`sub.sakrylle.com`）与 Sakrylle Image（`image.sakrylle.com`）为**线上生产服务**。
- 任何涉及生产数据库、nginx 配置、Docker 容器、Redis 或 TLS 证书的操作步骤，文档中均标注「需额外审批」，**不得依据本文档直接在生产环境执行**。
- 文档引用的代码路径均为本地仓库只读调研结果，不代表已部署的服务状态。
- 如需变更生产配置，须通过 `CLAUDE.md` 中规定的 ops 流程（SSH + docker compose + 审批）执行。

---

## 文档列表与阅读顺序

推荐阅读顺序：先建立 API/OIDC 基座理解（00-05），再按产品线展开（CLI 10-11 → Studio 20-21 → Web 30-31 → Chat 40-41 → Image 50-51），最后阅读跨生态综述（90-93）与 DESIGN.md。

### 元文档

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| — | `README.md`（本文） | 文档集导航，列出全部文档与阅读顺序 | 已完成 |
| 00 | `00-executive-summary.md` | 给决策者的执行摘要：目标、6 产品、路线图、最高优先级、最大风险 | 已完成 |
| 01 | `01-repositories-inventory.md` | 6 个仓库清单：路径、fork remote、分支、技术栈、上线状态、关键文件 | 已完成 |
| — | `DESIGN.md` | 全生态架构设计纲要：产品矩阵、OIDC 基座、数据流、安全边界、演进阶段 | 已完成 |

### 核心基座：Sakrylle API（sub2api fork）

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 02 | `02-sakrylle-api-oauth-current-state.md` | OAuth 2.0 现状报告：已实现端点、token 架构、与 OIDC 的差距分析 | 已完成 |
| 03 | `03-sakrylle-api-oidc-architecture.md` | OIDC 基座架构方案：RS256 密钥管理、id_token、JWKS、discovery、UserInfo 端点 | 已完成 |
| 04 | `04-oauth-oidc-commercial-capabilities.md` | OAuth/OIDC 与商业能力边界：哪些放 claims、哪些必须实时查、API Key 共存策略 | 已完成 |
| 05 | `05-configuration-isolation-standard.md` | 全生态配置隔离规范：`~/.sakrylle/`、`~/.sakrylle-cli/`、`SAKRYLLE_*` 变量全集、bundle id 命名 | 已完成 |

### Sakrylle CLI（OpenAI Codex CLI fork）

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 10 | `10-sakrylle-cli-research.md` | CLI 上游（codex-rs）现状调研：架构、配置目录、认证机制、Responses API 现状与兼容性风险 | 已完成 |
| 11 | `11-sakrylle-cli-development-plan.md` | CLI 改造计划：配置隔离、endpoint 适配、品牌替换、OIDC/Device Flow 接入分阶段方案 | 已完成 |

### Sakrylle Studio（CodexMonitor fork）

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 20 | `20-sakrylle-studio-research.md` | Studio 上游（CodexMonitor Tauri 2）现状调研：架构、认证透传、品牌点、配置路径 | 已完成 |
| 21 | `21-sakrylle-studio-development-plan.md` | Studio 改造计划：bundle id、Sentry 替换、updater endpoint、localStorage 前缀迁移 | 已完成 |

### Sakrylle Web（open-webui fork）

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 30 | `30-sakrylle-web-research.md` | Web 上游（open-webui SvelteKit+FastAPI）现状调研：OIDC 接入能力、品牌改造点、WEBUI_NAME 缺陷 | 已完成 |
| 31 | `31-sakrylle-web-development-plan.md` | Web 改造计划：env 配置、品牌替换、OIDC SSO 接入（依赖 OIDC 基座）、DATA_DIR 隔离 | 已完成 |

### Sakrylle Chat（kelivo fork）

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 40 | `40-sakrylle-chat-research.md` | Chat 上游（kelivo Flutter）现状调研：API Key 存储、provider 配置、主题系统、bundle id | 已完成 |
| 41 | `41-sakrylle-chat-development-plan.md` | Chat 改造计划：bundle id、Monet purple 主题、Sakrylle provider 预置、OAuth PKCE 接入 | 已完成 |

### Sakrylle Image（gpt_image_playground fork）

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 50 | `50-sakrylle-image-research.md` | Image 现状调研：OAuth PKCE 实现、localStorage 架构、OIDC 差距（id_token/nonce/scope）| 已完成 |
| 51 | `51-sakrylle-image-oidc-upgrade-plan.md` | Image OIDC 升级方案：scope 加 openid、id_token 解析、nonce、vite-env 补声明 | 已完成 |

### 跨生态综述与运维

| 编号 | 文件 | 一句话用途 | 状态 |
|---|---|---|---|
| 90 | `90-roadmap.md` | 全生态路线图：Phase 0–4、关键路径、并行/串行关系、里程碑与回滚原则 | 已完成 |
| 91 | `91-risk-register.md` | 风险登记册：OIDC 私钥、scope enforcement、Image 灰度、CLI 配置污染、Responses API 兼容性等风险与缓解 | 已完成 |
| 92 | `92-open-questions.md` | 开放问题清单：30 个问题的 resolved/code-check 状态、已冻结决策与实现期自查项 | 已完成 |
| 93 | `93-implementation-checklist.md` | 实施 checklist：Phase 0–4 跨产品任务、审批动作、验收标准、进度模型 | 已完成 |

---

## 文档间关键依赖

```
03（OIDC 基座） ← 所有产品 RP 接入文档均依赖此文档
     │
     ├── 11（CLI）      ── 依赖 OIDC Device Flow 端点
     ├── 21（Studio）   ── 依赖 OIDC auth_code+PKCE loopback
     ├── 31（Web）      ── 依赖 /.well-known/openid-configuration + UserInfo
     ├── 41（Chat）     ── 依赖 OIDC auth_code+PKCE + 自定义 scheme
     └── 51（Image）    ── 依赖 id_token + openid scope（已有 OAuth PKCE 基础）

05（配置隔离）  ← 所有客户端改造文档均引用此规范
04（商业边界）  ← 所有 claims 设计、token 形态决策的参考基准
```

---

## 文档格式约定

每篇研究/规划文档包含以下章节（视适用性取舍）：

1. 调研范围
2. 关键结论
3. 相关文件路径（research 文档须带 `path:line`）
4. 当前实现摘要
5. 差距分析
6. 开发任务拆分
7. 优先级
8. 风险
9. 验收标准
10. 后续问题

任务使用以下 checklist 格式：

```markdown
- [ ] 任务名称
  - 目标:
  - 涉及文件:
  - 实施说明:
  - 验收标准:
```

触及生产的步骤以「⚠ 需额外审批」标注。串行/并行关系在任务列表中明确标注。
