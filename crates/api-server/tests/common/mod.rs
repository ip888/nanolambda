//! Common test utilities

use nanolambda_api::{ApiServer, handlers::{InvokeRequest, InvokeResponse, CreateFunctionRequest, FunctionResponse}};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct TestServer {
    server: Arc<ApiServer>,
}

impl TestServer {
    pub async fn new() -> Self {
        let server = ApiServer::new_in_memory().await.expect("Failed to create test server");
        Self {
            server: Arc::new(server),
        }
    }
    
    pub async fn invoke_function(&self, function_name: &str, payload: Value) -> InvokeResponse {
        use axum::extract::{Path, State};
        use axum::Json;
        
        let state = State(self.server.clone());
        let path = Path(function_name.to_string());
        let request = Json(InvokeRequest {
            payload: serde_json::from_value(payload.get("payload").unwrap_or(&Value::Null).clone())
                .unwrap_or_default(),
        });
        
        let result = nanolambda_api::handlers::invoke_function(state, path, request).await;
        match result {
            Ok(Json(response)) => response,
            Err((status, Json(error))) => panic!("Invoke failed with status {}: {}", status, error.error),
        }
    }
    
    pub async fn create_function(&self, config: Value) -> Value {
        use axum::extract::State;
        use axum::Json;
        
        let state = State(self.server.clone());
        let config_request: CreateFunctionRequest = 
            serde_json::from_value(config).expect("Invalid function config");
        
        let result = nanolambda_api::handlers::create_function(state, Json(config_request)).await;
        match result {
            Ok(Json(response)) => serde_json::to_value(response).unwrap(),
            Err((status, Json(error))) => panic!("Create failed with status {}: {}", status, error.error),
        }
    }
}

/// Helper to create test function code
pub fn create_test_function(body: &str) -> String {
    format!(r#"
def handler(event, context):
    {}
"#, body)
}

/// Helper to assert metrics are valid
pub fn assert_valid_metrics(execution_ms: u64, total_ms: u64, memory_peak_mb: f64) {
    assert!(execution_ms >= 0, "Execution time must be non-negative");
    assert!(total_ms >= execution_ms, "Total time must be >= execution time");
    assert!(memory_peak_mb > 0.0, "Memory must be positive");
}
