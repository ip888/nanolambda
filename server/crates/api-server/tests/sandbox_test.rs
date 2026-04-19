//! Integration tests for the sandbox invoke and Prometheus metrics endpoints.
//!
//! Each test spins up a real HTTP server on a random port backed by an
//! in-memory `ApiServer`, then exercises the endpoint with `reqwest`.

use std::sync::Arc;

use nanolambda_api::ApiServer;
use serde_json::json;

/// Build an in-memory server, bind it to a random port, and return the base URL
/// together with a `JoinHandle` that drives the server in the background.
async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Build the same public routes the production server exposes.
    let app = axum::Router::new()
        .route(
            "/sandbox/invoke",
            axum::routing::post(nanolambda_api::sandbox_handlers::sandbox_invoke),
        )
        .route(
            "/metrics/prometheus",
            axum::routing::get(nanolambda_api::handlers::get_metrics_prometheus),
        )
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");

    let handle = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    (base_url, handle)
}

#[tokio::test]
async fn test_execute_python_hello() {
    let (base, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/sandbox/invoke"))
        .json(&json!({
            "tool": "execute_python",
            "args": { "code": "print('hello')" }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["stdout"].as_str().unwrap_or("").contains("hello"),
        "stdout should contain 'hello', got: {body}"
    );
    assert_eq!(body["exit_code"], 0);
}

#[tokio::test]
async fn test_execute_python_error() {
    let (base, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/sandbox/invoke"))
        .json(&json!({
            "tool": "execute_python",
            "args": { "code": "def (" }
        }))
        .send()
        .await
        .unwrap();

    // The server may return 200 with a non-zero exit_code, or 500.
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await.unwrap();
    if status == 200 {
        assert_ne!(body["exit_code"], 0, "syntax error should have non-zero exit_code");
    }
    // A 500 is also acceptable — the executor surfaced the error.
}

#[tokio::test]
async fn test_execute_shell_echo() {
    let (base, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/sandbox/invoke"))
        .json(&json!({
            "tool": "execute_shell",
            "args": { "command": "echo hi" }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["stdout"].as_str().unwrap_or("").contains("hi"),
        "stdout should contain 'hi', got: {body}"
    );
    assert_eq!(body["exit_code"], 0);
}

#[tokio::test]
async fn test_write_read_roundtrip() {
    let (base, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();

    // Write a file into the sandbox.
    let write_resp = client
        .post(format!("{base}/sandbox/invoke"))
        .json(&json!({
            "tool": "write_file",
            "args": {
                "path": "/workspace/roundtrip.txt",
                "content": "round-trip-payload"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(write_resp.status(), 200);
    let write_body: serde_json::Value = write_resp.json().await.unwrap();
    assert_eq!(write_body["exit_code"], 0, "write should succeed: {write_body}");

    // Read it back.
    let read_resp = client
        .post(format!("{base}/sandbox/invoke"))
        .json(&json!({
            "tool": "read_file",
            "args": { "path": "/workspace/roundtrip.txt" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(read_resp.status(), 200);
    let read_body: serde_json::Value = read_resp.json().await.unwrap();
    assert_eq!(read_body["exit_code"], 0, "read should succeed: {read_body}");
    assert!(
        read_body["stdout"]
            .as_str()
            .unwrap_or("")
            .contains("round-trip-payload"),
        "read should return written content, got: {read_body}"
    );
}

#[tokio::test]
async fn test_path_traversal_blocked() {
    let (base, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/sandbox/invoke"))
        .json(&json!({
            "tool": "read_file",
            "args": { "path": "/etc/passwd" }
        }))
        .send()
        .await
        .unwrap();

    // The sandbox _resolve() raises PermissionError which surfaces as
    // exit_code != 0 with the error in stderr.
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await.unwrap();

    if status == 200 {
        assert_ne!(body["exit_code"], 0, "path traversal should be blocked");
        let stderr = body["stderr"].as_str().unwrap_or("");
        assert!(
            stderr.contains("escapes sandbox") || stderr.contains("PermissionError"),
            "stderr should mention sandbox escape, got: {stderr}"
        );
    }
    // A 4xx/5xx is also acceptable — the handler rejected the request.
}

#[tokio::test]
async fn test_prometheus_metrics() {
    let (base, _handle) = spawn_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/metrics/prometheus"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("nanolambda_invocations_total"),
        "Prometheus output should contain nanolambda_invocations_total, got: {body}"
    );
}
