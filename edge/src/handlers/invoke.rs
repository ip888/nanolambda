//! Function invocation handler

use uuid::Uuid;
use web_time::Instant;
use worker::{Context, Env, Request, Response};

use crate::error::EdgeError;
use crate::handlers::functions::fetch_function;
use crate::runtime::quickjs::{JsVal, QuickJsConfig, QuickJsEngine};
use crate::types::{AuthContext, InvokeRequest, InvokeResponse, InvokeStatus, Permission, Runtime};

/// Invoke a function
pub async fn invoke_function(
    id: &str,
    mut req: Request,
    env: &Env,
    _ctx: &Context,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::FunctionsInvoke) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing functions:invoke permission".to_string(),
        ));
    }

    let start = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    // Parse invoke request
    let invoke_req: InvokeRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid invoke request: {e}")))?;

    // Fetch the function
    let function = fetch_function(id, env).await?;

    // Check ownership (or admin)
    if function.owner_id != auth.user_id && !auth.has_permission(&Permission::Admin) {
        return Err(EdgeError::AuthorizationDenied(
            "You don't have access to this function".to_string(),
        ));
    }

    // Determine timeout
    let timeout_ms = invoke_req.timeout_ms.unwrap_or(function.timeout_ms);

    // Execute based on runtime
    let result = match function.runtime {
        Runtime::JavaScript => {
            execute_javascript(
                &function.code,
                &function.handler,
                &invoke_req.payload,
                timeout_ms,
            )
            .await
        }
        Runtime::Python => {
            // Python via Pyodide - future implementation
            Err(EdgeError::NotImplemented(
                "Python runtime not yet available".to_string(),
            ))
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // Build response
    let (status, result_value, error) = match result {
        Ok(value) => (InvokeStatus::Success, Some(value), None),
        Err(e) => match e {
            EdgeError::Timeout(_) => (InvokeStatus::Timeout, None, Some(e.to_string())),
            _ => (InvokeStatus::Error, None, Some(e.to_string())),
        },
    };

    // Calculate billed duration (rounded up to nearest 1ms, minimum 1ms)
    let billed_duration_ms = duration_ms.max(1);

    let response = InvokeResponse {
        request_id,
        status,
        result: result_value,
        error,
        duration_ms,
        billed_duration_ms,
        memory_used_mb: function.memory_mb,
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Execute JavaScript function code using QuickJS engine
async fn execute_javascript(
    code: &str,
    handler: &str,
    payload: &serde_json::Value,
    timeout_ms: u64,
) -> Result<serde_json::Value, EdgeError> {
    // Configure QuickJS engine
    let config = QuickJsConfig {
        memory_limit: 64 * 1024 * 1024, // 64 MB for edge functions
        timeout_ms: timeout_ms as u32,
        max_stack_depth: 500,
        enable_console: true,
        enable_timers: true,
        enable_fetch: true,
    };

    // Create engine instance
    let mut engine = QuickJsEngine::new(config);

    // Convert payload to JsVal
    let event = JsVal::from_json(payload.clone());

    // Execute the code with the handler and event
    let result = engine.execute(code, Some(handler), Some(event));

    match result {
        Ok(js_result) => {
            if js_result.success {
                Ok(serde_json::json!({
                    "result": js_result.value,
                    "console_logs": js_result.console_logs,
                    "stats": {
                        "execution_time_ms": js_result.stats.execution_time_ms,
                        "memory_used": js_result.stats.memory_used,
                        "function_calls": js_result.stats.function_calls
                    }
                }))
            } else {
                Err(EdgeError::ExecutionFailed(
                    js_result
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string()),
                ))
            }
        }
        Err(e) => match e {
            crate::runtime::quickjs::QuickJsError::Timeout { elapsed_ms } => {
                Err(EdgeError::Timeout(elapsed_ms as u64))
            }
            crate::runtime::quickjs::QuickJsError::OutOfMemory { used, limit } => {
                Err(EdgeError::ExecutionFailed(format!(
                    "Out of memory: used {} bytes of {} limit",
                    used, limit
                )))
            }
            other => Err(EdgeError::ExecutionFailed(other.to_string())),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_request_parsing() {
        let json = r#"{"payload": {"key": "value"}, "async_invoke": false}"#;
        let req: InvokeRequest = serde_json::from_str(json).unwrap();
        assert!(!req.async_invoke);
        assert_eq!(req.payload["key"], "value");
    }
}
