//! SAKRYLLE: /v1/responses 契约测试 —— 断言 CLI 出站请求命中 Responses 端点、
//! 携带 Bearer 凭据、请求体符合 Responses wire API 形态。

use core_test_support::responses;
use core_test_support::test_codex::test_codex;

/// Stack size (8 MiB) for tokio worker threads.
///
/// The codex orchestration path allocates large futures on the stack.  In
/// unoptimised (debug) builds this can exceed the default tokio worker-thread
/// stack (2 MiB), causing a SIGABRT.  Using `thread_stack_size` on the tokio
/// runtime builder ensures every worker thread starts with enough head-room.
const TEST_STACK_SIZE_BYTES: usize = 8 * 1024 * 1024;

#[test]
fn sakrylle_responses_request_hits_endpoint_with_bearer_and_input() {
    core_test_support::skip_if_no_network!();

    let handle = std::thread::Builder::new()
        .name("sakrylle_responses_contract".to_string())
        .stack_size(TEST_STACK_SIZE_BYTES)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(TEST_STACK_SIZE_BYTES)
                .enable_all()
                .build()
                .expect("build tokio runtime")
                .block_on(run_test())
        })
        .expect("spawn large-stack thread");

    // Panics from assertions inside run_test propagate through the join.
    handle.join().expect("large-stack thread panicked")
}

async fn run_test() {
    let server = responses::start_mock_server().await;
    let response_body = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_completed("resp-1"),
    ]);
    let recorder = responses::mount_sse_once(&server, response_body).await;

    let test = test_codex().build(&server).await.expect("build test codex");
    test.submit_turn("hello").await.expect("submit turn");

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
