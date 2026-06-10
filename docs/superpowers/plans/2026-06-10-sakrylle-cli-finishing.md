# Sakrylle CLI 收尾 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 Sakrylle CLI 的品牌化收尾（4 处用户可见文案 + npm 子包映射 + 文档同步）与 OIDC 验证收尾（修复 revoke 失败单测、补 ES256/算法降级覆盖、补 `/v1/responses` mock 契约测试）。

**Architecture:** 双轨并行。轨道 1（品牌化）是纯字符串/打包/文档改动，零逻辑变更，低风险可先合。轨道 2（OIDC 验证）在仓库已有的 `wiremock` + `core_test_support` 测试脚手架上扩展，全部 mock、可进 CI、零真实凭据。两轨文件零重叠，可各自独立成 PR。

**Tech Stack:** Rust（edition 2024，cargo + nextest + insta）、Node（ESM）、wiremock、jsonwebtoken。命令统一走 `just`（见 CLAUDE.md）。

**关键事实（已核验 2026-06-10）：**
- revoke 失败根因：`codex-rs/login/tests/suite/logout.rs` 的 `body_json::<Value>()` 断言 revoke 请求体是 **JSON**，但 `codex-rs/login/src/auth/revoke.rs:112-117` 按 RFC 7009 正确发送 **form-encoded** 体。代码是对的，**测试断言过时**——修测试。
- 现有测试脚手架：`login` crate 已在 `oidc.rs` / `revoke.rs` / `tests/suite/*` 用 `wiremock`；`core` crate 的 `/v1/responses` 契约可用 `core_test_support::{responses, test_codex}` 驱动（见 `codex-rs/core/tests/responses_headers.rs` 范式，`test_codex().build(&server)` 默认用 `CodexAuth::from_api_key("dummy")`）。
- ES256 测试夹具：本计划内嵌一份**真实** P-256 私钥 PEM 与其公钥 JWK（x/y），无需占位。

---

## File Structure

| 文件 | 动作 | 职责 |
|---|---|---|
| `codex-rs/login/src/device_code_auth.rs` | 改 (L201-209) | device code 登录提示文案去品牌 |
| `codex-rs/tui/src/tooltips.rs` | 改 (L462) | 公告样例文案去品牌 |
| `codex-rs/tui/src/onboarding/auth.rs` | 改 (L445, L471) | 登录选项/禁用提示文案去品牌 |
| `codex-rs/cloud-tasks/src/lib.rs` | 改 (L80, L92) | 未登录提示文案 + 命令名去品牌 |
| `codex-cli/bin/codex.js` | 改 (L16-23) | npm 平台子包映射改为 `@sakrylle/cli-*` |
| `oidc-docs/implementation-status.md` | 改 | 状态与代码对齐 |
| `codex-rs/login/tests/suite/logout.rs` | 改 (L60-74, L160-174) | revoke 请求体断言：JSON → form-encoded |
| `codex-rs/login/src/oidc.rs` | 改 (tests mod) | 新增 ES256 正向 + HS256 拒绝 + nonce 不符拒绝 单测 + ES256 夹具 |
| `codex-rs/core/tests/sakrylle_responses_contract.rs` | 建 | `/v1/responses` mock 契约集成测试 |

---

## Task 0: 建立工作分支

**Files:** 无（git 操作）

- [ ] **Step 1: 从最新 main 建分支**

当前默认工作分支是 `sakrylle/main`。为收尾工作建独立分支：

```bash
cd "/Users/cervine/Documents/Sakrylle/Sakrylle CLI"
git checkout -b sakrylle/finishing-oidc-branding
```

Expected: 切到新分支，`git status` 干净。

---

# 轨道 1 — 品牌化收尾（PR-1，低风险，可先合）

> 轨道 1 全是字符串/打包/文档改动，无单元可 TDD；每个任务的“验证”=精确改动 + 构建/快照/grep 校验 + 提交。

## Task 1: device code 登录提示去品牌

**Files:**
- Modify: `codex-rs/login/src/device_code_auth.rs:203-209`

- [ ] **Step 1: 替换 `print_device_code_prompt` 文案**

把现有 `println!`（L203-209）中第一段品牌行改为 Sakrylle。当前：

```rust
    println!(
        "\nWelcome to Codex [v{ANSI_GRAY}{version}{ANSI_RESET}]\n{ANSI_GRAY}OpenAI's command-line coding agent{ANSI_RESET}\n\
\nFollow these steps to sign in with ChatGPT using device code authorization:\n\
```

改为：

```rust
    println!(
        "\nWelcome to Sakrylle [v{ANSI_GRAY}{version}{ANSI_RESET}]\n{ANSI_GRAY}Sakrylle CLI{ANSI_RESET}\n\
\nFollow these steps to sign in with Sakrylle using device code authorization:\n\
```

（其余行 L206-208 不变。）

- [ ] **Step 2: 构建校验**

Run: `just clippy -p codex-login`
Expected: 编译通过、无新告警。

- [ ] **Step 3: 提交**

```bash
git add codex-rs/login/src/device_code_auth.rs
git commit -m "brand(login): rebrand device code prompt to Sakrylle"
```

## Task 2: TUI 文案去品牌（tooltips + onboarding）

**Files:**
- Modify: `codex-rs/tui/src/tooltips.rs:462`
- Modify: `codex-rs/tui/src/onboarding/auth.rs:445,471`

- [ ] **Step 1: 改 tooltips 公告样例**

`codex-rs/tui/src/tooltips.rs:462`，当前：

```
content = "Welcome to Codex! Check out the new onboarding flow."
```

改为：

```
content = "Welcome to Sakrylle! Check out the new onboarding flow."
```

- [ ] **Step 2: 改登录选项标签** `codex-rs/tui/src/onboarding/auth.rs:445`，当前：

```rust
                        "Sign in with ChatGPT",
```

改为：

```rust
                        "Sign in with Sakrylle",
```

- [ ] **Step 3: 改禁用提示** `codex-rs/tui/src/onboarding/auth.rs:471`，当前：

```rust
                "  API key login is disabled by this workspace. Sign in with ChatGPT to continue."
```

改为：

```rust
                "  API key login is disabled by this workspace. Sign in with Sakrylle to continue."
```

- [ ] **Step 4: 跑 TUI 测试并检查快照差异**

Run: `just test -p codex-tui`
然后：`cargo insta pending-snapshots -p codex-tui`
Expected: 若有快照因文案变化而 pending，逐一确认是预期的品牌文案变化后执行 `cargo insta accept -p codex-tui`；否则无 pending。

- [ ] **Step 5: 提交**

```bash
git add codex-rs/tui/src/tooltips.rs codex-rs/tui/src/onboarding/auth.rs
git add codex-rs/tui/src/**/snapshots/ 2>/dev/null || true
git commit -m "brand(tui): rebrand onboarding/tooltip strings to Sakrylle"
```

## Task 3: cloud-tasks 未登录提示去品牌 + 命令名修正

**Files:**
- Modify: `codex-rs/cloud-tasks/src/lib.rs:80,92`（两处文案完全相同）

- [ ] **Step 1: 替换两处提示**

L80 与 L92 当前均为：

```rust
                "Not signed in. Please run 'codex login' to sign in with ChatGPT, then re-run 'codex cloud'."
```

两处都改为（`codex`→`sakrylle`、`ChatGPT`→`Sakrylle`；子命令名 `cloud` 已由 `main.rs` 定义，正确）：

```rust
                "Not signed in. Please run 'sakrylle login' to sign in with Sakrylle, then re-run 'sakrylle cloud'."
```

- [ ] **Step 2: 构建校验**

Run: `just clippy -p codex-cloud-tasks`
Expected: 编译通过。（若 crate 名不确定，用 `cargo build -p codex-cloud-tasks` 前先 `rg '^name' codex-rs/cloud-tasks/Cargo.toml` 确认包名。）

- [ ] **Step 3: 提交**

```bash
git add codex-rs/cloud-tasks/src/lib.rs
git commit -m "brand(cloud-tasks): rebrand sign-in hint and fix command name"
```

## Task 4: npm 平台子包映射改为 `@sakrylle/cli-*`

**Files:**
- Modify: `codex-cli/bin/codex.js:16-23`

- [ ] **Step 1: 替换映射表**

当前（L16-23）：

```js
const PLATFORM_PACKAGE_BY_TARGET = {
  "x86_64-unknown-linux-musl": "@openai/codex-linux-x64",
  "aarch64-unknown-linux-musl": "@openai/codex-linux-arm64",
  "x86_64-apple-darwin": "@openai/codex-darwin-x64",
  "aarch64-apple-darwin": "@openai/codex-darwin-arm64",
  "x86_64-pc-windows-msvc": "@openai/codex-win32-x64",
  "aarch64-pc-windows-msvc": "@openai/codex-win32-arm64",
};
```

改为：

```js
const PLATFORM_PACKAGE_BY_TARGET = {
  "x86_64-unknown-linux-musl": "@sakrylle/cli-linux-x64",
  "aarch64-unknown-linux-musl": "@sakrylle/cli-linux-arm64",
  "x86_64-apple-darwin": "@sakrylle/cli-darwin-x64",
  "aarch64-apple-darwin": "@sakrylle/cli-darwin-arm64",
  "x86_64-pc-windows-msvc": "@sakrylle/cli-win32-x64",
  "aarch64-pc-windows-msvc": "@sakrylle/cli-win32-arm64",
};
```

- [ ] **Step 2: 语法校验**

Run: `node --check codex-cli/bin/codex.js`
Expected: 无输出（语法正确）。

- [ ] **Step 3: 提交**

```bash
git add codex-cli/bin/codex.js
git commit -m "brand(npm): map platform sub-packages to @sakrylle/cli-*"
```

> 注：实际把这 6 个子包发布到 registry 属生产动作，见末尾“手动验收清单”。

## Task 5: 全仓品牌残留扫描（验证轨道 1 完整）

**Files:** 无（校验）

- [ ] **Step 1: 扫描用户可见上游文案**

Run:
```bash
rg -n "Welcome to Codex|OpenAI's command-line|sign in with ChatGPT|@openai/codex-" \
  codex-rs/ codex-cli/ --glob '!**/*_tests.rs' --glob '!**/tests/**'
```
Expected: **无匹配**（内部符号如 `SignInState::ChatGpt`、`uses_codex_backend` 等不在此扫描范围，属预期保留）。若有匹配，回到对应 Task 补改。

## Task 6: 同步 oidc-docs 文档

**Files:**
- Modify: `oidc-docs/implementation-status.md`

- [ ] **Step 1: 更新状态描述**

把顶部状态行 `Phase 1–2 complete ... Phase 3–5 remain.` 改为反映现状：Phase 3 代码已完成、待真实环境验证。具体：
- 第 11 行 `Current status:` 改为：`**Phase 1–3 代码完成（OIDC 验证全链路 + 品牌化主体）。真实环境验证与生产发布为手动收尾项。**`
- “Phase 3” 小节标题由 `❌ Not started` 改为 `⚠️ 代码完成，待真实环境契约验证`，并注明已补 `/v1/responses` mock 契约测试（见本计划 Task 9）。
- “Phase 4: Brand cleanup” 小节：把 device code prompt / CLI help / README 三条 ⚠️ 勾为 ✅（本计划 Track 1 已改）。
- “Test status” 表：`logout/revoke` 行由 `2 | 3` 改为 `5 | 0`，备注改为“revoke 请求体断言由 JSON 修正为 form-encoded（RFC 7009）”。

- [ ] **Step 2: 提交**

```bash
git add oidc-docs/implementation-status.md
git commit -m "docs(oidc): sync implementation-status with current code state"
```

---

# 轨道 2 — OIDC 验证收尾（PR-2，扩展 wiremock 脚手架）

## Task 7: 修复 revoke 失败单测（请求体 JSON → form-encoded）

**Files:**
- Modify: `codex-rs/login/tests/suite/logout.rs:60-74,160-174`

- [ ] **Step 1: 先复现失败，确认根因**

Run: `just test -p codex-login -- logout`
Expected: 在有网络环境下，`logout_with_revoke_revokes_refresh_token_then_removes_auth` 与 `auth_manager_logout_with_revoke_uses_cached_auth` 因 `body_json::<Value>()` 解析 form-encoded 体失败而 FAIL。（无网络则 `skip_if_no_network!` 跳过——本任务需在有网环境验证。）

- [ ] **Step 2: 改第一处断言（L60-74）**

当前：

```rust
    let requests = server
        .received_requests()
        .await
        .context("failed to fetch revoke requests")?;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .body_json::<Value>()
            .context("revoke request should be JSON")?,
        json!({
            "token": REFRESH_TOKEN,
            "token_type_hint": "refresh_token",
            "client_id": CLIENT_ID,
        })
    );
    server.verify().await;
```

改为（断言 form-encoded 体；按 `&`/`=` 拆分后排序比对，值均为 unreserved 字符无需解码）：

```rust
    let requests = server
        .received_requests()
        .await
        .context("failed to fetch revoke requests")?;
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8(requests[0].body.clone())
        .context("revoke request body should be UTF-8")?;
    let mut params: Vec<(&str, &str)> = body
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .collect();
    params.sort();
    let mut expected = vec![
        ("client_id", CLIENT_ID),
        ("token", REFRESH_TOKEN),
        ("token_type_hint", "refresh_token"),
    ];
    expected.sort();
    assert_eq!(params, expected);
    server.verify().await;
```

- [ ] **Step 3: 改第二处断言（L160-174，`auth_manager_logout_with_revoke_uses_cached_auth`）**

该处断言块与 Step 2 的“当前”代码完全相同，做**完全相同**的替换（替换为 Step 2 的“改为”代码）。

- [ ] **Step 4: 清理未使用 import**

`logout.rs` 顶部若 `use serde_json::Value;` 在替换后不再被使用，删除该行（保留 `use serde_json::json;`，`chatgpt_auth` 等仍用）。先 `rg 'Value' codex-rs/login/tests/suite/logout.rs` 确认无残留引用再删。

- [ ] **Step 5: 跑测试确认全绿**

Run: `just test -p codex-login -- logout`
Expected: 3 个 logout 测试全部 PASS（有网环境）。

- [ ] **Step 6: 提交**

```bash
git add codex-rs/login/tests/suite/logout.rs
git commit -m "test(login): assert form-encoded revoke body per RFC 7009"
```

## Task 8: 补 ES256 正向 + 算法降级拒绝 + nonce 不符拒绝 单测

**Files:**
- Modify: `codex-rs/login/src/oidc.rs`（`#[cfg(test)] mod tests`，在现有测试后追加夹具与测试）

> 现有单测已覆盖：RS256 正向、错误 issuer、错误 audience、缺 nonce、discovery 不匹配。本任务补 ES256 正向（此前完全未测）、HS256 拒绝（验证 `ALLOWED_ID_TOKEN_ALGS` 防降级）、nonce 不符拒绝。

- [ ] **Step 1: 在 tests mod 顶部补 ES256 夹具**

在 `oidc.rs` tests mod 内（紧接 `const NONCE` 之后）新增以下夹具。PEM 与 JWK 为真实 P-256 密钥对（已用 openssl 生成）：

```rust
    const ES256_KID: &str = "es256-test-key";

    // 真实 P-256 私钥（PKCS#8），仅用于测试。
    const TEST_EC_PRIVATE_KEY_PEM: &[u8] = br#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgSDKKdXs1zAc8PT1U
J9F2jGuJosAvKOsfvG/RD7EHYwihRANCAASAtsayQTFIuklZWia6MtUuHIFhvPVK
fU5UloEKyhNtleNJcA/rW8ZLVY334pB0n8lHwMCuhojRPbeEff5UclTu
-----END PRIVATE KEY-----"#;

    fn es256_jwks_body() -> serde_json::Value {
        json!({
            "keys": [{
                "kty": "EC",
                "crv": "P-256",
                "alg": "ES256",
                "use": "sig",
                "kid": ES256_KID,
                "x": "gLbGskExSLpJWVomujLVLhyBYbz1Sn1OVJaBCsoTbZU",
                "y": "40lwD-tbxktVjffikHSfyUfAwK6GiNE9t4R9_lRyVO4"
            }]
        })
    }

    fn es256_discovery(server: &MockServer) -> OidcDiscovery {
        let mut d = discovery(server);
        d.id_token_signing_alg_values_supported = Some(vec!["ES256".to_string()]);
        d
    }

    async fn mount_es256_jwks(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/oauth/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(es256_jwks_body()))
            .mount(server)
            .await;
    }

    fn signed_es256_id_token(
        issuer: &str,
        audience: serde_json::Value,
        nonce: Option<&str>,
    ) -> jsonwebtoken::errors::Result<String> {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(ES256_KID.to_string());
        let now = Utc::now().timestamp() as usize;
        let mut claims = json!({
            "iss": issuer,
            "sub": "user-123",
            "aud": audience,
            "iat": now,
            "exp": now + 3600,
        });
        if let Some(nonce) = nonce {
            claims["nonce"] = json!(nonce);
        }
        jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_ec_pem(TEST_EC_PRIVATE_KEY_PEM)?,
        )
    }
```

- [ ] **Step 2: 写 ES256 正向测试**

```rust
    #[tokio::test]
    async fn verifies_valid_es256_id_token() {
        let server = MockServer::start().await;
        mount_es256_jwks(&server).await;
        let discovery = es256_discovery(&server);
        let token = signed_es256_id_token(&discovery.issuer, json!(CLIENT_ID), Some(NONCE))
            .expect("signed es256 token");

        let verified = verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect("valid ES256 token should verify");

        assert_eq!(verified.claims.sub, "user-123");
        assert_eq!(verified.claims.iss, discovery.issuer);
    }
```

- [ ] **Step 3: 写 HS256 拒绝测试（防算法降级）**

```rust
    #[tokio::test]
    async fn rejects_hs256_id_token() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);

        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("test-key".to_string());
        let now = Utc::now().timestamp() as usize;
        let claims = json!({
            "iss": discovery.issuer,
            "sub": "user-123",
            "aud": CLIENT_ID,
            "iat": now,
            "exp": now + 3600,
            "nonce": NONCE,
        });
        let token = jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"shared-secret"),
        )
        .expect("signed hs256 token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("HS256 is not an allowed id_token algorithm");
    }
```

- [ ] **Step 4: 写 nonce 不符拒绝测试**

```rust
    #[tokio::test]
    async fn rejects_nonce_mismatch() {
        let server = MockServer::start().await;
        mount_jwks(&server).await;
        let discovery = discovery(&server);
        let token = signed_id_token(&discovery.issuer, json!(CLIENT_ID), Some("other-nonce"))
            .expect("signed token");

        verify_id_token(&token, &discovery, CLIENT_ID, Some(NONCE))
            .await
            .expect_err("mismatched nonce should fail");
    }
```

- [ ] **Step 5: 核对 refresh rotation 拒绝已被覆盖（不重复造测试）**

spec 2c 要求“refresh rotation 后旧 token 被拒”有覆盖。先盘点现有 `auth_refresh` 套件：

Run:
```bash
rg -n "rotat|reuse|replay|old.*refresh|invalid_grant" codex-rs/login/tests/suite/auth_refresh.rs
```
判定：
- 若已有断言“旧/已轮换的 refresh_token 再用被拒（如 `invalid_grant`）”，则**无需新增**，在提交信息中注明“refresh rotation reject 已由 auth_refresh 覆盖”。
- 若**缺失**，在 `auth_refresh.rs` 仿照该文件现有 wiremock 范式补一个测试：mock `/oauth/token` 对旧 refresh_token 返回 `400 {"error":"invalid_grant"}`，断言 `AuthManager` 刷新失败并提示重新登录。（仅在确认缺失时执行，避免重复。）

- [ ] **Step 6: 跑测试确认全绿**

Run: `just test -p codex-login -- oidc`
Expected: 含新增 3 个在内的 oidc 单测全部 PASS（共 8 个）。

- [ ] **Step 7: fmt + 提交**

```bash
just fmt
git add codex-rs/login/src/oidc.rs
git add codex-rs/login/tests/suite/auth_refresh.rs 2>/dev/null || true
git commit -m "test(login): cover ES256 verify, HS256 rejection, nonce mismatch"
```

## Task 9: `/v1/responses` mock 契约集成测试

**Files:**
- Create: `codex-rs/core/tests/sakrylle_responses_contract.rs`

> 复用 `core_test_support`（见 `codex-rs/core/tests/responses_headers.rs` 范式）。`test_codex().build(&server)` 把 provider base_url 指向 `{server}/v1`、wire_api=Responses，并默认用 `CodexAuth::from_api_key("dummy")`，故出站请求带 `Authorization: Bearer dummy`。请求被 responses mock 捕获即证明命中 `/v1/responses`。

- [ ] **Step 1: 新建契约测试文件**

```rust
//! SAKRYLLE: /v1/responses 契约测试 —— 断言 CLI 出站请求命中 Responses 端点、
//! 携带 Bearer 凭据、请求体符合 Responses wire API 形态。

use core_test_support::responses;
use core_test_support::test_codex::test_codex;

#[tokio::test]
async fn sakrylle_responses_request_hits_endpoint_with_bearer_and_input() {
    core_test_support::skip_if_no_network!();

    let server = responses::start_mock_server().await;
    let response_body = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_completed("resp-1"),
    ]);
    let recorder = responses::mount_sse_once(&server, response_body).await;

    let test = test_codex()
        .build(&server)
        .await
        .expect("build test codex");
    test.submit_turn("hello")
        .await
        .expect("submit turn");

    // 命中 /v1/responses（被该 mount 捕获即证明路径正确）+ 携带 Bearer 凭据。
    let request = recorder.single_request();
    let auth = request
        .header("authorization")
        .expect("responses request must carry an Authorization header");
    assert!(
        auth.starts_with("Bearer "),
        "Authorization must be a Bearer token, got: {auth}"
    );

    // Responses wire API 形态：请求体含 `input` 数组。
    let body = request.body_json();
    assert!(
        body.get("input").is_some(),
        "responses request body must include `input`, got: {body}"
    );
}
```

- [ ] **Step 2: 跑测试确认通过**

Run: `just test -p codex-core -- sakrylle_responses_contract`
Expected: PASS（有网环境）。若 `single_request()` / `mount_sse_once` / `submit_turn` 签名与 `responses_headers.rs` 当前不一致，以 `codex-rs/core/tests/responses_headers.rs` 中的实际用法为准对齐（该文件是同范式的可编译参照）。

- [ ] **Step 3: fmt + 提交**

```bash
just fmt
git add codex-rs/core/tests/sakrylle_responses_contract.rs
git commit -m "test(core): add /v1/responses mock contract test for Sakrylle"
```

## Task 10: 轨道 2 收口校验

**Files:** 无（校验）

- [ ] **Step 1: 跑两 crate 全量相关测试**

Run:
```bash
just test -p codex-login
just test -p codex-core
```
Expected: 全绿。logout/revoke 5 个全过；oidc 8 个全过；新增契约测试过。

- [ ] **Step 2: fmt-check + clippy**

Run:
```bash
just fmt-check
just clippy -p codex-login
just clippy -p codex-core
```
Expected: 无格式问题、无新 clippy 告警。

---

## 附录 A — 手动验收清单（不进 CI，需外部资源/审批）

> 这些项依赖真实凭据、生产审批或多平台机器，**不在本计划的代码任务内**，交由后续手动执行。

- [ ] 真实端到端：非生产 profile 下 `sakrylle login`（一键）+ `--device` + refresh + `logout`，全程零复制粘贴。
- [ ] 真实计费：`SAKRYLLE_API_KEY=<Claude-group key> sakrylle exec "…"` → sub2api `usage_logs` 落 `total_cost>0`。
- [ ] **生产 `oauth_clients`（需审批）**：核对 `sakrylle-cli` 的 `redirect_uris` 含通配 loopback（`http://127.0.0.1:*/callback` + `http://localhost`），缺则补 seed。
- [ ] 三平台实测：macOS / Linux / Windows 浏览器拉起 + loopback 回调；SSH 远程 `--device`。
- [ ] **npm 发布（需审批）**：`@sakrylle/cli-<platform>` 6 个子包发布到 registry，`npx @sakrylle/cli login` 跑通。

## 附录 B — Definition of Done

- 轨道 1：Task 5 扫描无用户可见上游文案匹配；`node --check` 通过；oidc-docs 与代码一致。
- 轨道 2：`just test -p codex-login` 与 `just test -p codex-core` 全绿；`just fmt-check` + clippy 干净。
- 附录 A 清单已记录、明确标注“需审批 / 需真实环境”，交后续执行。
