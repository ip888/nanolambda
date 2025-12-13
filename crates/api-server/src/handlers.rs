//! Request handlers - Integrated with storage and runtime

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    response::Html,
};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, error};
use uuid::Uuid;

use crate::ApiServer;
use nanolambda_storage::{FunctionConfig as StorageFunctionConfig, InvocationRecord, InvocationStatus};
use nanolambda_runtime::{GenericFunctionConfig, Language};
use nanolambda_runtime::runtime_trait::Runtime;

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFunctionRequest {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub code: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListFunctionsResponse {
    pub functions: Vec<FunctionResponse>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeRequest {
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvokeResponse {
    pub request_id: String,
    pub status_code: u16,
    pub body: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub execution_time_ms: u64,
    pub memory_used_mb: f64,
    pub cold_start: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Create a new function
pub async fn create_function(
    State(state): State<Arc<ApiServer>>,
    Json(request): Json<CreateFunctionRequest>,
) -> Result<Json<FunctionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Creating function: {}", request.name);
    
    // Create function config for storage
    let config = StorageFunctionConfig {
        name: request.name.clone(),
        runtime: request.runtime.clone(),
        handler: request.handler.clone(),
        code: request.code,
        memory_mb: request.memory_mb,
        timeout_ms: request.timeout_ms,
        environment: request.environment,
    };
    
    // Store function in database
    match state.storage().create_function(config) {
        Ok(_function_id) => {
            // Retrieve the created function to get all fields
            match state.storage().get_function(&request.name) {
                Ok(Some(function)) => {
                    Ok(Json(FunctionResponse {
                        name: function.name,
                        runtime: function.runtime,
                        handler: function.handler,
                        memory_mb: function.memory_mb,
                        timeout_ms: function.timeout_ms,
                        status: function.status.as_str().to_string(),
                        created_at: function.created_at,
                        updated_at: function.updated_at,
                    }))
                }
                Ok(None) => {
                    error!("Function created but not found: {}", request.name);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "InternalError".to_string(),
                            message: "Function created but not retrievable".to_string(),
                        }),
                    ))
                }
                Err(e) => {
                    error!("Failed to retrieve created function: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "InternalError".to_string(),
                            message: format!("Failed to retrieve function: {}", e),
                        }),
                    ))
                }
            }
        }
        Err(e) => {
            error!("Failed to create function: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "CreateFailed".to_string(),
                    message: format!("Failed to create function: {}", e),
                }),
            ))
        }
    }
}

/// List all functions
pub async fn list_functions(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<ListFunctionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Listing all functions");
    
    match state.storage().list_functions() {
        Ok(functions) => {
            let function_responses: Vec<FunctionResponse> = functions
                .into_iter()
                .map(|f| FunctionResponse {
                    name: f.name,
                    runtime: f.runtime,
                    handler: f.handler,
                    memory_mb: f.memory_mb,
                    timeout_ms: f.timeout_ms,
                    status: f.status.as_str().to_string(),
                    created_at: f.created_at,
                    updated_at: f.updated_at,
                })
                .collect();
            
            let count = function_responses.len();
            Ok(Json(ListFunctionsResponse {
                functions: function_responses,
                count,
            }))
        }
        Err(e) => {
            error!("Failed to list functions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "ListFailed".to_string(),
                    message: format!("Failed to list functions: {}", e),
                }),
            ))
        }
    }
}

/// Get a specific function
pub async fn get_function(
    State(state): State<Arc<ApiServer>>,
    Path(name): Path<String>,
) -> Result<Json<FunctionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting function: {}", name);
    
    match state.storage().get_function(&name) {
        Ok(Some(function)) => {
            Ok(Json(FunctionResponse {
                name: function.name,
                runtime: function.runtime,
                handler: function.handler,
                memory_mb: function.memory_mb,
                timeout_ms: function.timeout_ms,
                status: function.status.as_str().to_string(),
                created_at: function.created_at,
                updated_at: function.updated_at,
            }))
        }
        Ok(None) => {
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "FunctionNotFound".to_string(),
                    message: format!("Function '{}' not found", name),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to get function: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "GetFailed".to_string(),
                    message: format!("Failed to get function: {}", e),
                }),
            ))
        }
    }
}

/// Update an existing function
pub async fn update_function(
    State(state): State<Arc<ApiServer>>,
    Path(name): Path<String>,
    Json(request): Json<CreateFunctionRequest>,
) -> Result<Json<FunctionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating function: {}", name);
    
    // Create function config for storage
    let config = StorageFunctionConfig {
        name: request.name,
        runtime: request.runtime,
        handler: request.handler,
        code: request.code,
        memory_mb: request.memory_mb,
        timeout_ms: request.timeout_ms,
        environment: request.environment,
    };
    
    // Update function in database
    match state.storage().update_function(&name, config) {
        Ok(_) => {
            // Retrieve the updated function
            match state.storage().get_function(&name) {
                Ok(Some(function)) => {
                    Ok(Json(FunctionResponse {
                        name: function.name,
                        runtime: function.runtime,
                        handler: function.handler,
                        memory_mb: function.memory_mb,
                        timeout_ms: function.timeout_ms,
                        status: function.status.as_str().to_string(),
                        created_at: function.created_at,
                        updated_at: function.updated_at,
                    }))
                }
                Ok(None) => {
                    Err((
                        StatusCode::NOT_FOUND,
                        Json(ErrorResponse {
                            error: "FunctionNotFound".to_string(),
                            message: format!("Function '{}' not found after update", name),
                        }),
                    ))
                }
                Err(e) => {
                    error!("Failed to retrieve updated function: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "InternalError".to_string(),
                            message: format!("Failed to retrieve function: {}", e),
                        }),
                    ))
                }
            }
        }
        Err(e) => {
            error!("Failed to update function: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "UpdateFailed".to_string(),
                    message: format!("Failed to update function: {}", e),
                }),
            ))
        }
    }
}

/// Delete a function
pub async fn delete_function(
    State(state): State<Arc<ApiServer>>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting function: {}", name);
    
    match state.storage().delete_function(&name) {
        Ok(_) => {
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete function: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "DeleteFailed".to_string(),
                    message: format!("Failed to delete function: {}", e),
                }),
            ))
        }
    }
}

/// Handler for function invocation - INTEGRATED WITH STORAGE + RUNTIME + CONCURRENCY + RATE LIMITING
pub async fn invoke_function(
    State(state): State<Arc<ApiServer>>,
    Path(name): Path<String>,
    req: axum::extract::Request,
) -> Result<Json<InvokeResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Invoking function: {}", name);
    
    let request_id = Uuid::new_v4().to_string();
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    // Extract auth context and request body
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    // Check trial status (block if expired)
    if !api_key.is_empty() {
        if let Some(trial_mgr) = state.trial_manager() {
            match trial_mgr.get_trial_status(&api_key).await {
                Ok(trial) => {
                    if !trial.is_valid() {
                        let reason = trial.expiration_reason().unwrap_or_else(|| "Trial expired".to_string());
                        return Err((
                            StatusCode::PAYMENT_REQUIRED,
                            Json(ErrorResponse {
                                error: "TrialExpired".to_string(),
                                message: format!("{} - Please upgrade to continue using the service", reason),
                            }),
                        ));
                    }
                }
                Err(_) => {
                    // No trial found - could be paid user or legacy key, allow through
                }
            }
        }
    }
    
    // Parse request body
    let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Failed to read request body: {}", e),
            }),
        ))?;
    
    let request: InvokeRequest = serde_json::from_slice(&bytes)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Invalid JSON: {}", e),
            }),
        ))?;
    
    // 0a. Check tier limits and monthly quota
    if !api_key.is_empty() {
        if let Some(tier_mgr) = state.tier_manager() {
            // Check monthly invocation limit
            match tier_mgr.check_monthly_limit(&api_key).await {
                Ok(within_limit) => {
                    if !within_limit {
                        return Err((
                            StatusCode::PAYMENT_REQUIRED,
                            Json(ErrorResponse {
                                error: "MonthlyLimitExceeded".to_string(),
                                message: "Monthly invocation limit reached for your tier. Please upgrade to continue.".to_string(),
                            }),
                        ));
                    }
                }
                Err(_) => {
                    // No tier assigned yet - will be assigned after trial
                }
            }
        }
    }
    
    // 0b. Check rate limit
    if let Err(e) = state.rate_limiter().check_rate_limit(&api_key).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "RateLimitExceeded".to_string(),
                message: format!("{}", e),
            }),
        ));
    }
    
    // 0c. Acquire concurrency permits (queues if at limit, rejects if queue full)
    let _concurrency_guard = match state.concurrency().acquire(&name).await {
        Ok(guard) => guard,
        Err(e) => {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    error: "ConcurrencyLimitReached".to_string(),
                    message: format!("Function '{}' is at capacity: {}", name, e),
                }),
            ));
        }
    };
    
    // 1. Load function from database
    let function = match state.storage().get_function(&name) {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "FunctionNotFound".to_string(),
                    message: format!("Function '{}' not found", name),
                }),
            ));
        }
        Err(e) => {
            error!("Failed to get function from storage: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "StorageError".to_string(),
                    message: format!("Failed to load function: {}", e),
                }),
            ));
        }
    };
    
    // 2. Check tier limits for memory and timeout
    if !api_key.is_empty() {
        if let Some(tier_mgr) = state.tier_manager() {
            if let Ok(user_tier) = tier_mgr.get_user_tier(&api_key).await {
                let tier_config = tier_mgr.get_tier_config(user_tier.tier).await;
                if let Err(e) = tier_config.check_limits(
                    function.memory_mb.try_into().unwrap_or(u32::MAX), 
                    function.timeout_ms.try_into().unwrap_or(i64::MAX)
                ) {
                    return Err((
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse {
                            error: "TierLimitExceeded".to_string(),
                            message: e,
                        }),
                    ));
                }
            }
        }
    }
    
    // 3. Check if function is active
    if function.status != nanolambda_storage::FunctionStatus::Active {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "FunctionNotActive".to_string(),
                message: format!("Function '{}' is not active", name),
            }),
        ));
    }
    
    // 3. Detect language from runtime
    let language = if function.runtime.starts_with("python") {
        Language::Python
    } else if function.runtime.starts_with("nodejs") {
        Language::NodeJS
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "UnsupportedRuntime".to_string(),
                message: format!("Runtime '{}' is not supported", function.runtime),
            }),
        ));
    };
    
    // 4. Build configuration based on runtime type
    let execution_result = match language {
        Language::Python => {
            // Python uses synchronous FunctionConfig
            let py_config = nanolambda_runtime::FunctionConfig {
                id: function.id,
                version: function.version,
                name: function.name.clone(),
                code: function.code.clone(),
                handler: function.handler.clone(),
                environment: function.environment.clone(),
                memory_limit_mb: function.memory_mb,
                timeout_seconds: function.timeout_ms / 1000,
                working_dir: None,
            };
            
            // Clone the payload for the blocking task
            let payload = request.payload.clone();
            let executor = Arc::clone(state.python_executor());
            
            tokio::task::spawn_blocking(move || {
                // This runs in a blocking thread pool
                let runtime = tokio::runtime::Handle::try_current().ok();
                let exec = if let Some(rt) = runtime {
                    rt.block_on(executor.lock())
                } else {
                    // Fallback: create a new runtime for the blocking task
                    tokio::runtime::Runtime::new().unwrap().block_on(executor.lock())
                };
                exec.execute(py_config, payload)
            }).await
                .map_err(|e| (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "ExecutionError".to_string(),
                        message: format!("Task join error: {}", e),
                    }),
                ))?
        }
        Language::NodeJS => {
            // Node.js implements Runtime trait with GenericFunctionConfig
            let config = GenericFunctionConfig::new(
                function.name.clone(),
                Language::NodeJS,
                function.code.clone(),
                function.handler.clone(),
            )
            .with_memory_limit(function.memory_mb)
            .with_timeout(function.timeout_ms / 1000);
            
            let executor = state.nodejs_executor().lock().await;
            executor.execute(&config, request.payload.clone()).await
        }
        Language::Java => {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                Json(ErrorResponse {
                    error: "NotImplemented".to_string(),
                    message: "Java runtime not yet implemented".to_string(),
                }),
            ));
        }
    };
    
    // 5. Process execution result
    match execution_result {
        Ok(exec_result) => {
            let completed_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            
            // Record invocation in database
            let invocation_record = InvocationRecord {
                function_id: function.id,
                request_id: request_id.clone(),
                status: if exec_result.success {
                    InvocationStatus::Success
                } else {
                    InvocationStatus::Error
                },
                started_at,
                completed_at: Some(completed_at),
                execution_time_ms: Some(exec_result.metrics.execution_ms as i64),
                memory_used_mb: Some(exec_result.metrics.memory_peak_mb as i64),
                cold_start: exec_result.metrics.is_cold_start,
                error_message: exec_result.error.clone(),
            };
            
            if let Err(e) = state.storage().record_invocation(invocation_record) {
                error!("Failed to record invocation: {}", e);
                // Continue anyway - execution succeeded
            }
            
            // Record metrics
            let metrics_point = crate::metrics::MetricPoint {
                timestamp: completed_at,
                function_name: name.clone(),
                cold_start: exec_result.metrics.is_cold_start,
                execution_time_ms: exec_result.metrics.execution_ms as i64,
                status: if exec_result.success {
                    crate::metrics::MetricStatus::Success
                } else {
                    crate::metrics::MetricStatus::Error
                },
            };
            state.metrics().record(metrics_point).await;
            
            // Record usage for billing (in-memory tracker)
            let usage_record = crate::usage_tracker::UsageRecord {
                timestamp: completed_at,
                api_key: api_key.clone(),
                function_name: name.clone(),
                execution_time_ms: exec_result.metrics.execution_ms as i64,
                memory_used_mb: exec_result.metrics.memory_peak_mb,
                cold_start: exec_result.metrics.is_cold_start,
                success: exec_result.success,
            };
            state.usage_tracker().record(usage_record).await;
            
            // Also record to persistent database (async, non-blocking)
            if let Some(usage_db) = state.usage_db() {
                // Get tier-specific pricing (if user has a tier), otherwise use dynamic pricing
                let (invocation_cost, compute_cost) = if let Some(tier_mgr) = state.tier_manager() {
                    if let Ok(user_tier) = tier_mgr.get_user_tier(&api_key).await {
                        let tier_config = tier_mgr.get_tier_config(user_tier.tier).await;
                        let inv_cost = tier_config.calculate_invocation_cost(1);
                        let comp_cost = tier_config.calculate_compute_cost(
                            exec_result.metrics.memory_peak_mb as u32,
                            exec_result.metrics.execution_ms as i64
                        );
                        (inv_cost, comp_cost)
                    } else {
                        // No tier assigned, use dynamic pricing
                        let pricing = if let Some(pricing_mgr) = state.pricing() {
                            pricing_mgr.get_pricing().await
                        } else {
                            nanolambda_storage::pricing::PricingConfig::default()
                        };
                        let inv_cost = pricing.calculate_invocation_cost(1);
                        let comp_cost = pricing.calculate_compute_cost(
                            exec_result.metrics.memory_peak_mb as u32,
                            exec_result.metrics.execution_ms as i64
                        );
                        (inv_cost, comp_cost)
                    }
                } else {
                    // No tier manager, use dynamic pricing
                    let pricing = if let Some(pricing_mgr) = state.pricing() {
                        pricing_mgr.get_pricing().await
                    } else {
                        nanolambda_storage::pricing::PricingConfig::default()
                    };
                    let inv_cost = pricing.calculate_invocation_cost(1);
                    let comp_cost = pricing.calculate_compute_cost(
                        exec_result.metrics.memory_peak_mb as u32,
                        exec_result.metrics.execution_ms as i64
                    );
                    (inv_cost, comp_cost)
                };
                
                let total_cost = invocation_cost + compute_cost;
                
                let event = nanolambda_storage::usage_db::UsageEvent {
                    id: None,
                    timestamp: completed_at,
                    api_key: api_key.clone(),
                    function_name: name.clone(),
                    request_id: request_id.clone(),
                    execution_time_ms: exec_result.metrics.execution_ms as i64,
                    memory_mb: exec_result.metrics.memory_peak_mb as u32,
                    cold_start: exec_result.metrics.is_cold_start,
                    success: exec_result.success,
                    invocation_cost,
                    compute_cost,
                    total_cost,
                };
                usage_db.record_event(event);
            }
            
            // Increment trial and tier counters (async, non-blocking)
            if !api_key.is_empty() {
                if let Some(trial_mgr) = state.trial_manager() {
                    let trial_mgr_clone = trial_mgr.clone();
                    let api_key_clone = api_key.clone();
                    tokio::spawn(async move {
                        if let Err(e) = trial_mgr_clone.increment_invocation(&api_key_clone).await {
                            error!("Failed to increment trial counter: {}", e);
                        }
                    });
                }
                
                if let Some(tier_mgr) = state.tier_manager() {
                    let tier_mgr_clone = tier_mgr.clone();
                    let api_key_clone = api_key.clone();
                    tokio::spawn(async move {
                        if let Err(e) = tier_mgr_clone.increment_monthly_invocations(&api_key_clone).await {
                            error!("Failed to increment tier monthly counter: {}", e);
                        }
                    });
                }
            }
            
            // Parse result string to JSON
            let result_value = if let Some(ref result_str) = exec_result.result {
                serde_json::from_str(result_str).unwrap_or(serde_json::Value::String(result_str.clone()))
            } else {
                serde_json::Value::Null
            };
            
            // Return response
            if exec_result.success {
                Ok(Json(InvokeResponse {
                    request_id,
                    status_code: 200,
                    body: result_value,
                    error: None,
                    metrics: ExecutionMetrics {
                        execution_time_ms: exec_result.metrics.execution_ms,
                        memory_used_mb: exec_result.metrics.memory_peak_mb,
                        cold_start: exec_result.metrics.is_cold_start,
                    },
                }))
            } else {
                Ok(Json(InvokeResponse {
                    request_id,
                    status_code: 500,
                    body: serde_json::Value::Null,
                    error: exec_result.error,
                    metrics: ExecutionMetrics {
                        execution_time_ms: exec_result.metrics.execution_ms,
                        memory_used_mb: exec_result.metrics.memory_peak_mb,
                        cold_start: exec_result.metrics.is_cold_start,
                    },
                }))
            }
        }
        Err(e) => {
            error!("Function execution error: {}", e);
            
            // Record failed invocation
            let completed_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            
            let invocation_record = InvocationRecord {
                function_id: function.id,
                request_id: request_id.clone(),
                status: InvocationStatus::Error,
                started_at,
                completed_at: Some(completed_at),
                execution_time_ms: None,
                memory_used_mb: None,
                cold_start: false,
                error_message: Some(e.to_string()),
            };
            
            if let Err(err) = state.storage().record_invocation(invocation_record) {
                error!("Failed to record failed invocation: {}", err);
            }
            
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "ExecutionError".to_string(),
                    message: format!("Function execution failed: {}", e),
                }),
            ))
        }
    }
}

/// Health check endpoint
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// ============================================================================
// Versioning Endpoints
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishVersionRequest {
    pub runtime: String,
    pub handler: String,
    pub code: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionResponse {
    pub id: i64,
    pub name: String,
    pub version: i64,
    pub is_latest: bool,
    pub runtime: String,
    pub handler: String,
    pub code_hash: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListVersionsResponse {
    pub versions: Vec<VersionResponse>,
    pub count: usize,
}

/// List all versions of a function
pub async fn list_function_versions(
    State(state): State<Arc<ApiServer>>,
    Path(name): Path<String>,
) -> Result<Json<ListVersionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Listing versions for function: {}", name);
    
    match state.storage().list_function_versions(&name) {
        Ok(functions) => {
            let versions: Vec<VersionResponse> = functions
                .into_iter()
                .map(|f| VersionResponse {
                    id: f.id,
                    name: f.name,
                    version: f.version,
                    is_latest: f.is_latest,
                    runtime: f.runtime,
                    handler: f.handler,
                    code_hash: f.code_hash,
                    memory_mb: f.memory_mb,
                    timeout_ms: f.timeout_ms,
                    created_at: f.created_at,
                    updated_at: f.updated_at,
                })
                .collect();
            
            let count = versions.len();
            Ok(Json(ListVersionsResponse { versions, count }))
        }
        Err(e) => {
            error!("Failed to list versions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "StorageError".to_string(),
                    message: format!("Failed to list versions: {}", e),
                }),
            ))
        }
    }
}

/// Get a specific version of a function
pub async fn get_function_version(
    State(state): State<Arc<ApiServer>>,
    Path((name, version)): Path<(String, i64)>,
) -> Result<Json<VersionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting function {} version {}", name, version);
    
    match state.storage().get_function_by_version(&name, version) {
        Ok(Some(f)) => Ok(Json(VersionResponse {
            id: f.id,
            name: f.name,
            version: f.version,
            is_latest: f.is_latest,
            runtime: f.runtime,
            handler: f.handler,
            code_hash: f.code_hash,
            memory_mb: f.memory_mb,
            timeout_ms: f.timeout_ms,
            created_at: f.created_at,
            updated_at: f.updated_at,
        })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "VersionNotFound".to_string(),
                message: format!("Function '{}' version {} not found", name, version),
            }),
        )),
        Err(e) => {
            error!("Failed to get version: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "StorageError".to_string(),
                    message: format!("Failed to get version: {}", e),
                }),
            ))
        }
    }
}

/// Publish a new version of a function
pub async fn publish_function_version(
    State(state): State<Arc<ApiServer>>,
    Path(name): Path<String>,
    Json(request): Json<PublishVersionRequest>,
) -> Result<Json<VersionResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Publishing new version for function: {}", name);
    
    // Create config for new version
    let config = StorageFunctionConfig {
        name: name.clone(),
        runtime: request.runtime,
        handler: request.handler,
        code: request.code,
        memory_mb: request.memory_mb,
        timeout_ms: request.timeout_ms,
        environment: request.environment,
    };
    
    match state.storage().publish_version(&name, config) {
        Ok(new_id) => {
            info!("Published new version for function '{}' with id {}", name, new_id);
            
            // Get the newly created version to return
            match state.storage().get_function(&name) {
                Ok(Some(f)) => Ok(Json(VersionResponse {
                    id: f.id,
                    name: f.name,
                    version: f.version,
                    is_latest: f.is_latest,
                    runtime: f.runtime,
                    handler: f.handler,
                    code_hash: f.code_hash,
                    memory_mb: f.memory_mb,
                    timeout_ms: f.timeout_ms,
                    created_at: f.created_at,
                    updated_at: f.updated_at,
                })),
                Ok(None) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "InternalError".to_string(),
                        message: "Version created but could not be retrieved".to_string(),
                    }),
                )),
                Err(e) => {
                    error!("Failed to retrieve new version: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "StorageError".to_string(),
                            message: format!("Version created but could not be retrieved: {}", e),
                        }),
                    ))
                }
            }
        }
        Err(e) => {
            error!("Failed to publish version: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "StorageError".to_string(),
                    message: format!("Failed to publish version: {}", e),
                }),
            ))
        }
    }
}

// ============================================================================
// API Key Management Handlers
// ============================================================================

use nanolambda_storage::{CreateApiKeyRequest, ApiKey};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateKeyResponse {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListKeysResponse {
    pub keys: Vec<ApiKeyInfo>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: i64,
    pub name: String,
    pub permissions: Vec<String>,
    pub status: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    // Don't expose the actual key in list operations
}

impl From<ApiKey> for ApiKeyInfo {
    fn from(key: ApiKey) -> Self {
        ApiKeyInfo {
            id: key.id,
            name: key.name,
            permissions: key.permissions,
            status: key.status.as_str().to_string(),
            created_at: key.created_at,
            expires_at: key.expires_at,
            last_used_at: key.last_used_at,
        }
    }
}

/// POST /auth/keys - Create new API key
pub async fn create_api_key(
    State(state): State<Arc<ApiServer>>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage().create_api_key(request) {
        Ok(key) => {
            info!("Created API key '{}' with id {}", key.name, key.id);
            
            // Auto-start trial for new API keys
            if let Some(trial_mgr) = state.trial_manager() {
                match trial_mgr.start_trial(&key.key).await {
                    Ok(trial) => {
                        info!("Started 14-day trial for API key {}: {} days, {} invocations remaining", 
                            key.id, trial.days_remaining, trial.invocations_remaining);
                    }
                    Err(e) => {
                        // Log error but don't fail key creation
                        error!("Failed to start trial for API key {}: {}", key.id, e);
                    }
                }
            }
            
            Ok(Json(CreateKeyResponse {
                id: key.id,
                key: key.key,
                name: key.name,
                permissions: key.permissions,
                created_at: key.created_at,
                expires_at: key.expires_at,
            }))
        }
        Err(e) => {
            error!("Failed to create API key: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "StorageError".to_string(),
                    message: format!("Failed to create API key: {}", e),
                }),
            ))
        }
    }
}

/// GET /auth/keys - List all API keys
pub async fn list_api_keys(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<ListKeysResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage().list_api_keys() {
        Ok(keys) => {
            let key_infos: Vec<ApiKeyInfo> = keys.into_iter().map(|k| k.into()).collect();
            let count = key_infos.len();
            Ok(Json(ListKeysResponse {
                keys: key_infos,
                count,
            }))
        }
        Err(e) => {
            error!("Failed to list API keys: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "StorageError".to_string(),
                    message: format!("Failed to list API keys: {}", e),
                }),
            ))
        }
    }
}

/// DELETE /auth/keys/{id} - Revoke API key
pub async fn revoke_api_key(
    State(state): State<Arc<ApiServer>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.storage().revoke_api_key(id) {
        Ok(_) => {
            info!("Revoked API key with id {}", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to revoke API key: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "NotFound".to_string(),
                    message: format!("API key not found: {}", e),
                }),
            ))
        }
    }
}

// ============================================================================
// Metrics & Observability Handlers
// ============================================================================

use crate::metrics::MetricsAggregate;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub last_hour: MetricsAggregate,
    pub last_24h: MetricsAggregate,
    pub all_time: MetricsAggregate,
}

/// GET /metrics - Get metrics data
pub async fn get_metrics(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<MetricsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let last_hour = state.metrics().get_metrics(3600).await;
    let last_24h = state.metrics().get_metrics(86400).await;
    let all_time = state.metrics().get_all_time_metrics().await;
    
    Ok(Json(MetricsResponse {
        last_hour,
        last_24h,
        all_time,
    }))
}

/// GET /dashboard - Serve metrics dashboard
pub async fn get_dashboard() -> Html<&'static str> {
    Html(include_str!("../dashboard.html"))
}

/// GET /concurrency - Get concurrency statistics
pub async fn get_concurrency_stats(
    State(state): State<Arc<ApiServer>>,
) -> Json<serde_json::Value> {
    let global_stats = state.concurrency().get_global_stats();
    let function_stats = state.concurrency().get_all_stats().await;
    
    Json(serde_json::json!({
        "global": {
            "max_concurrent": global_stats.max_global_concurrent,
            "current_running": global_stats.current_global_running,
            "max_per_function": global_stats.max_per_function,
            "max_queue_size": global_stats.max_queue_size,
        },
        "functions": function_stats,
    }))
}

// ============================================================================
// Rate Limiting Handlers
// ============================================================================

/// GET /rate-limit/status - Get rate limit status for current API key
pub async fn get_rate_limit_status(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    let status = state.rate_limiter().get_status(&api_key).await;
    
    Ok(Json(serde_json::json!({
        "tier": serde_json::to_value(&status.tier).unwrap(),
        "available_tokens": status.available_tokens,
        "capacity": status.capacity,
        "refill_rate": format!("{:.2}/sec", status.refill_rate),
        "refill_rate_per_min": format!("{:.0}/min", status.refill_rate * 60.0),
    })))
}

/// GET /rate-limit/stats - Get all rate limit statistics (admin only)
pub async fn get_all_rate_limit_stats(
    State(state): State<Arc<ApiServer>>,
) -> Json<serde_json::Value> {
    let all_stats = state.rate_limiter().get_all_stats().await;
    
    let stats_json: Vec<_> = all_stats
        .into_iter()
        .map(|(key, status)| {
            serde_json::json!({
                "api_key": key,
                "tier": serde_json::to_value(&status.tier).unwrap(),
                "available_tokens": status.available_tokens,
                "capacity": status.capacity,
                "refill_rate_per_min": format!("{:.0}/min", status.refill_rate * 60.0),
            })
        })
        .collect();
    
    Json(serde_json::json!({ "rate_limits": stats_json }))
}

/// PUT /rate-limit/tier - Set rate limit tier for an API key (admin only)
#[derive(Debug, Deserialize)]
pub struct SetTierRequest {
    pub api_key: String,
    pub tier: String,
}

pub async fn set_rate_limit_tier(
    State(state): State<Arc<ApiServer>>,
    Json(request): Json<SetTierRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    use crate::rate_limiter::RateLimitTier;
    
    let tier = match request.tier.to_lowercase().as_str() {
        "free" => RateLimitTier::Free,
        "hobby" => RateLimitTier::Hobby,
        "developer" => RateLimitTier::Developer,
        "production" => RateLimitTier::Production,
        "enterprise" => RateLimitTier::Enterprise,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidTier".to_string(),
                    message: format!("Unknown tier: {}. Valid: free, hobby, developer, production, enterprise", request.tier),
                }),
            ));
        }
    };
    
    state.rate_limiter().set_tier(&request.api_key, tier).await;
    
    Ok(StatusCode::OK)
}

// ============================================================================
// Usage Tracking & Billing Handlers
// ============================================================================

/// GET /usage/stats - Get usage stats for current API key
pub async fn get_usage_stats(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    match state.usage_tracker().get_stats(&api_key).await {
        Some(stats) => Ok(Json(serde_json::to_value(&stats).unwrap())),
        None => Ok(Json(serde_json::json!({
            "api_key": api_key,
            "total_invocations": 0,
            "message": "No usage recorded yet"
        }))),
    }
}

/// GET /usage/billing - Get billing information for current API key
pub async fn get_billing_info(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    match state.usage_tracker().get_stats(&api_key).await {
        Some(stats) => {
            let billing = crate::usage_tracker::UsageTracker::calculate_bill(&stats);
            Ok(Json(serde_json::to_value(&billing).unwrap()))
        },
        None => Ok(Json(serde_json::json!({
            "api_key": api_key,
            "total_cost": 0.0,
            "message": "No usage recorded yet"
        }))),
    }
}

/// GET /usage/all - Get usage stats for all API keys (admin)
pub async fn get_all_usage_stats(
    State(state): State<Arc<ApiServer>>,
) -> Json<serde_json::Value> {
    let all_stats = state.usage_tracker().get_all_stats().await;
    
    let stats_with_billing: Vec<_> = all_stats
        .iter()
        .map(|stats| {
            let billing = crate::usage_tracker::UsageTracker::calculate_bill(stats);
            serde_json::json!({
                "api_key": stats.api_key,
                "invocations": stats.total_invocations,
                "successful": stats.successful_invocations,
                "failed": stats.failed_invocations,
                "functions_used": stats.functions_used.len(),
                "total_cost": format!("${:.4}", billing.total_cost),
                "invocation_cost": format!("${:.4}", billing.invocation_cost),
                "memory_cost": format!("${:.4}", billing.memory_cost),
                "gb_seconds": format!("{:.2}", billing.gb_seconds),
            })
        })
        .collect();
    
    Json(serde_json::json!({
        "usage_stats": stats_with_billing,
        "total_customers": all_stats.len(),
    }))
}

// ============================================================================
// Pricing Management Handlers
// ============================================================================

/// GET /pricing - Get current pricing configuration
pub async fn get_pricing(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pricing_mgr) = state.pricing() {
        let pricing = pricing_mgr.get_pricing().await;
        Ok(Json(serde_json::json!({
            "version": pricing.version,
            "price_per_invocation": pricing.price_per_invocation,
            "price_per_1m_invocations": pricing.price_per_invocation * 1_000_000.0,
            "price_per_gb_second": pricing.price_per_gb_second,
            "effective_date": pricing.effective_date,
            "notes": pricing.notes,
            "is_active": pricing.is_active,
        })))
    } else {
        // Fallback for in-memory mode
        let default = nanolambda_storage::pricing::PricingConfig::default();
        Ok(Json(serde_json::json!({
            "version": default.version,
            "price_per_invocation": default.price_per_invocation,
            "price_per_1m_invocations": default.price_per_invocation * 1_000_000.0,
            "price_per_gb_second": default.price_per_gb_second,
            "notes": "In-memory mode: using default pricing",
        })))
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricingRequest {
    price_per_invocation: Option<f64>,
    price_per_gb_second: Option<f64>,
    notes: String,
}

/// PUT /pricing - Update pricing configuration (admin only)
pub async fn update_pricing(
    State(state): State<Arc<ApiServer>>,
    Json(request): Json<UpdatePricingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pricing_mgr) = state.pricing() {
        let current = pricing_mgr.get_pricing().await;
        
        let new_invocation_price = request.price_per_invocation.unwrap_or(current.price_per_invocation);
        let new_gb_second_price = request.price_per_gb_second.unwrap_or(current.price_per_gb_second);
        
        match pricing_mgr.update_pricing(new_invocation_price, new_gb_second_price, request.notes).await {
            Ok(pricing) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "Pricing updated successfully",
                "version": pricing.version,
                "price_per_invocation": pricing.price_per_invocation,
                "price_per_1m_invocations": pricing.price_per_invocation * 1_000_000.0,
                "price_per_gb_second": pricing.price_per_gb_second,
                "effective_date": pricing.effective_date,
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "PricingUpdateFailed".to_string(),
                    message: format!("Failed to update pricing: {}", e),
                }),
            )),
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PricingUnavailable".to_string(),
                message: "Pricing management not available in in-memory mode".to_string(),
            }),
        ))
    }
}

/// GET /pricing/history - Get pricing history
pub async fn get_pricing_history(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pricing_mgr) = state.pricing() {
        match pricing_mgr.get_pricing_history().await {
            Ok(history) => {
                let formatted: Vec<_> = history.iter().map(|p| {
                    serde_json::json!({
                        "version": p.version,
                        "price_per_invocation": p.price_per_invocation,
                        "price_per_1m_invocations": p.price_per_invocation * 1_000_000.0,
                        "price_per_gb_second": p.price_per_gb_second,
                        "effective_date": p.effective_date,
                        "notes": p.notes,
                        "is_active": p.is_active,
                    })
                }).collect();
                
                Ok(Json(serde_json::json!({
                    "history": formatted,
                    "count": history.len(),
                })))
            },
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "HistoryFetchFailed".to_string(),
                    message: format!("Failed to fetch pricing history: {}", e),
                }),
            )),
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PricingUnavailable".to_string(),
                message: "Pricing management not available in in-memory mode".to_string(),
            }),
        ))
    }
}


// ============================================================================
// Trial Period Management
// ============================================================================

/// GET /trial/status - Get trial status for authenticated user
pub async fn get_trial_status(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    if let Some(trial_mgr) = state.trial_manager() {
        match trial_mgr.get_trial_status(&api_key).await {
            Ok(trial) => {
                Ok(Json(serde_json::json!({
                    "trial_active": trial.trial_active,
                    "trial_valid": trial.is_valid(),
                    "trial_start": trial.trial_start,
                    "trial_end": trial.trial_end,
                    "days_remaining": trial.days_remaining,
                    "invocations_used": trial.invocations_used,
                    "invocations_remaining": trial.invocations_remaining,
                    "invocation_limit": 100_000,
                    "expiration_reason": trial.expiration_reason(),
                })))
            }
            Err(_) => {
                Ok(Json(serde_json::json!({
                    "trial_active": false,
                    "trial_valid": false,
                    "message": "No trial period found for this API key",
                })))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TrialUnavailable".to_string(),
                message: "Trial management not available in in-memory mode".to_string(),
            }),
        ))
    }
}

/// GET /trial/all - Get all active trials (admin)
pub async fn get_all_trials(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(trial_mgr) = state.trial_manager() {
        match trial_mgr.get_all_active_trials().await {
            Ok(trials) => {
                let formatted: Vec<_> = trials.iter().map(|t| {
                    serde_json::json!({
                        "api_key": t.api_key,
                        "trial_active": t.trial_active,
                        "trial_valid": t.is_valid(),
                        "trial_start": t.trial_start,
                        "trial_end": t.trial_end,
                        "days_remaining": t.days_remaining,
                        "invocations_used": t.invocations_used,
                        "invocations_remaining": t.invocations_remaining,
                        "expiration_reason": t.expiration_reason(),
                    })
                }).collect();
                
                Ok(Json(serde_json::json!({
                    "trials": formatted,
                    "count": trials.len(),
                })))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "TrialFetchFailed".to_string(),
                    message: format!("Failed to fetch trials: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TrialUnavailable".to_string(),
                message: "Trial management not available in in-memory mode".to_string(),
            }),
        ))
    }
}


// ============================================================================
// Tier Management
// ============================================================================

/// GET /tier/current - Get current tier for authenticated user
pub async fn get_current_tier(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    if let Some(tier_mgr) = state.tier_manager() {
        match tier_mgr.get_user_tier(&api_key).await {
            Ok(user_tier) => {
                let tier_config = tier_mgr.get_tier_config(user_tier.tier).await;
                Ok(Json(serde_json::json!({
                    "tier": tier_config.tier,
                    "name": tier_config.name,
                    "description": tier_config.description,
                    "pricing": {
                        "price_per_invocation": tier_config.price_per_invocation,
                        "price_per_1m_invocations": tier_config.price_per_invocation * 1_000_000.0,
                        "price_per_gb_second": tier_config.price_per_gb_second,
                    },
                    "limits": {
                        "max_invocations_per_month": tier_config.max_invocations_per_month,
                        "max_memory_mb": tier_config.max_memory_mb,
                        "max_timeout_ms": tier_config.max_timeout_ms,
                        "max_concurrent_executions": tier_config.max_concurrent_executions,
                    },
                    "features": {
                        "support_level": tier_config.support_level,
                        "custom_domains": tier_config.custom_domains,
                        "advanced_monitoring": tier_config.advanced_monitoring,
                        "priority_execution": tier_config.priority_execution,
                    },
                    "usage": {
                        "monthly_invocations": user_tier.monthly_invocations,
                        "assigned_at": user_tier.assigned_at,
                    }
                })))
            }
            Err(_) => {
                // No tier assigned yet (likely in trial)
                Ok(Json(serde_json::json!({
                    "tier": null,
                    "message": "No tier assigned. Currently in trial period."
                })))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TierUnavailable".to_string(),
                message: "Tier management not available in in-memory mode".to_string(),
            }),
        ))
    }
}

/// GET /tier/plans - Get all available tier plans
pub async fn get_tier_plans(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(tier_mgr) = state.tier_manager() {
        let tiers = tier_mgr.get_all_tiers().await;
        
        let plans: Vec<_> = tiers.iter().map(|t| {
            serde_json::json!({
                "tier": t.tier,
                "name": t.name,
                "description": t.description,
                "pricing": {
                    "price_per_invocation": t.price_per_invocation,
                    "price_per_1m_invocations": t.price_per_invocation * 1_000_000.0,
                    "price_per_gb_second": t.price_per_gb_second,
                },
                "limits": {
                    "max_invocations_per_month": t.max_invocations_per_month,
                    "max_memory_mb": t.max_memory_mb,
                    "max_timeout_ms": t.max_timeout_ms,
                    "max_concurrent_executions": t.max_concurrent_executions,
                },
                "features": {
                    "support_level": t.support_level,
                    "custom_domains": t.custom_domains,
                    "advanced_monitoring": t.advanced_monitoring,
                    "priority_execution": t.priority_execution,
                }
            })
        }).collect();
        
        Ok(Json(serde_json::json!({
            "plans": plans,
            "count": tiers.len(),
        })))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TierUnavailable".to_string(),
                message: "Tier management not available in in-memory mode".to_string(),
            }),
        ))
    }
}

/// PUT /tier/upgrade - Upgrade user to a higher tier
pub async fn upgrade_tier(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    // Parse request body
    let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Failed to read request body: {}", e),
            }),
        ))?;
    
    #[derive(serde::Deserialize)]
    struct UpgradeRequest {
        tier: String,
    }
    
    let upgrade_req: UpgradeRequest = serde_json::from_slice(&bytes)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Invalid JSON: {}", e),
            }),
        ))?;
    
    if let Some(tier_mgr) = state.tier_manager() {
        let new_tier = nanolambda_storage::tier::TierLevel::from_str(&upgrade_req.tier)
            .ok_or_else(|| (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidTier".to_string(),
                    message: format!("Invalid tier: {}. Must be 'starter', 'pro', or 'enterprise'", upgrade_req.tier),
                }),
            ))?;
        
        match tier_mgr.upgrade_tier(&api_key, new_tier).await {
            Ok(user_tier) => {
                let tier_config = tier_mgr.get_tier_config(user_tier.tier).await;
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("Successfully upgraded to {} tier", tier_config.name),
                    "tier": tier_config.tier,
                    "name": tier_config.name,
                    "assigned_at": user_tier.assigned_at,
                })))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "UpgradeFailed".to_string(),
                    message: format!("Failed to upgrade tier: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TierUnavailable".to_string(),
                message: "Tier management not available in in-memory mode".to_string(),
            }),
        ))
    }
}

/// GET /tier/recommendation - Get intelligent upgrade recommendation
pub async fn get_upgrade_recommendation(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    if let Some(tier_mgr) = state.tier_manager() {
        match tier_mgr.get_upgrade_recommendation(&api_key).await {
            Ok(Some(recommendation)) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "has_recommendation": true,
                    "recommendation": {
                        "current_tier": recommendation.current_tier.as_str(),
                        "recommended_tier": recommendation.recommended_tier.as_str(),
                        "urgency": recommendation.urgency,
                        "usage_percent": recommendation.usage_percent,
                        "current_usage": recommendation.current_usage,
                        "current_limit": recommendation.current_limit,
                        "reasons": recommendation.reasons,
                        "estimated_savings": recommendation.estimated_savings,
                    }
                })))
            }
            Ok(None) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "has_recommendation": false,
                    "message": "No upgrade recommended at this time"
                })))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "RecommendationFailed".to_string(),
                    message: format!("Failed to get recommendation: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TierUnavailable".to_string(),
                message: "Tier management not available".to_string(),
            }),
        ))
    }
}

/// GET /tier/preview - Preview upgrade benefits and costs
pub async fn get_upgrade_preview(
    State(state): State<Arc<ApiServer>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    let target_tier_str = params.get("tier").ok_or_else(|| (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "MissingParameter".to_string(),
            message: "Query parameter 'tier' is required".to_string(),
        }),
    ))?;
    
    if let Some(tier_mgr) = state.tier_manager() {
        let target_tier = nanolambda_storage::tier::TierLevel::from_str(target_tier_str)
            .ok_or_else(|| (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidTier".to_string(),
                    message: format!("Invalid tier: {}", target_tier_str),
                }),
            ))?;
        
        // Get current and target tier configs
        match tier_mgr.get_user_tier(&api_key).await {
            Ok(user_tier) => {
                let current_config = tier_mgr.get_tier_config(user_tier.tier).await;
                let target_config = tier_mgr.get_tier_config(target_tier).await;
                
                // Calculate differences
                let invocation_increase = if let (Some(current), Some(target)) = (
                    current_config.max_invocations_per_month,
                    target_config.max_invocations_per_month,
                ) {
                    target - current
                } else {
                    0
                };
                
                let memory_increase = target_config.max_memory_mb as i64 - current_config.max_memory_mb as i64;
                let timeout_increase = target_config.max_timeout_ms - current_config.max_timeout_ms;
                let concurrent_increase = target_config.max_concurrent_executions as i64 - current_config.max_concurrent_executions as i64;
                
                // Build features comparison
                let mut new_features = Vec::new();
                if !current_config.custom_domains && target_config.custom_domains {
                    new_features.push("Custom domains");
                }
                if !current_config.advanced_monitoring && target_config.advanced_monitoring {
                    new_features.push("Advanced monitoring");
                }
                if !current_config.priority_execution && target_config.priority_execution {
                    new_features.push("Priority execution");
                }
                if current_config.support_level != target_config.support_level {
                    new_features.push("Enhanced support");
                }
                
                Ok(Json(serde_json::json!({
                    "success": true,
                    "current": {
                        "tier": current_config.tier.as_str(),
                        "name": current_config.name,
                        "invocations_per_month": current_config.max_invocations_per_month,
                        "memory_mb": current_config.max_memory_mb,
                        "timeout_ms": current_config.max_timeout_ms,
                        "concurrent_executions": current_config.max_concurrent_executions,
                    },
                    "target": {
                        "tier": target_config.tier.as_str(),
                        "name": target_config.name,
                        "invocations_per_month": target_config.max_invocations_per_month,
                        "memory_mb": target_config.max_memory_mb,
                        "timeout_ms": target_config.max_timeout_ms,
                        "concurrent_executions": target_config.max_concurrent_executions,
                    },
                    "improvements": {
                        "invocation_increase": invocation_increase,
                        "memory_increase_mb": memory_increase,
                        "timeout_increase_ms": timeout_increase,
                        "concurrent_increase": concurrent_increase,
                        "new_features": new_features,
                    },
                    "current_usage": {
                        "monthly_invocations": user_tier.monthly_invocations,
                        "usage_percent": if let Some(limit) = current_config.max_invocations_per_month {
                            if limit > 0 {
                                ((user_tier.monthly_invocations as f64 / limit as f64) * 100.0) as u32
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    }
                })))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "PreviewFailed".to_string(),
                    message: format!("Failed to generate preview: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "TierUnavailable".to_string(),
                message: "Tier management not available".to_string(),
            }),
        ))
    }
}

// ============================================================================
// Invoice Endpoints
// ============================================================================

/// Get all invoices for the authenticated user
pub async fn list_invoices(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }

    // Get invoices from database
    match state.invoice_manager.list_invoices(&api_key, Some(100), None).await {
        Ok(invoices) => Ok(Json(json!({
            "success": true,
            "invoices": invoices,
            "count": invoices.len(),
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "InvoiceRetrievalFailed".to_string(),
                message: format!("Failed to retrieve invoices: {}", e),
            }),
        )),
    }
}

/// Get a specific invoice by ID
pub async fn get_invoice(
    State(state): State<Arc<ApiServer>>,
    Path(invoice_id): Path<i64>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }

    // Get invoice from database
    match state.invoice_manager.get_invoice(invoice_id).await {
        Ok(Some(invoice)) => {
            // Verify the invoice belongs to this user
            if invoice.api_key != api_key {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "Forbidden".to_string(),
                        message: "This invoice does not belong to you".to_string(),
                    }),
                ));
            }

            // Get line items
            let line_items = state.invoice_manager.get_line_items(invoice_id).await
                .unwrap_or_default();

            Ok(Json(json!({
                "success": true,
                "invoice": invoice,
                "line_items": line_items,
            })))
        },
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "NotFound".to_string(),
                message: "Invoice not found".to_string(),
            }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "InvoiceRetrievalFailed".to_string(),
                message: format!("Failed to retrieve invoice: {}", e),
            }),
        )),
    }
}

/// Get invoice summary for the authenticated user
pub async fn get_invoice_summary(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }

    // Get summary from database
    match state.invoice_manager.get_invoice_summary(&api_key).await {
        Ok(summary) => Ok(Json(json!({
            "success": true,
            "summary": summary,
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "SummaryRetrievalFailed".to_string(),
                message: format!("Failed to retrieve invoice summary: {}", e),
            }),
        )),
    }
}

/// Sync invoice from Stripe (refresh status and URLs)
pub async fn sync_invoice(
    State(state): State<Arc<ApiServer>>,
    Path(stripe_invoice_id): Path<String>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }

    // Sync invoice from Stripe
    match state.invoice_manager.sync_stripe_invoice(&stripe_invoice_id).await {
        Ok(Some(invoice)) => {
            // Verify the invoice belongs to this user
            if invoice.api_key != api_key {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: "Forbidden".to_string(),
                        message: "This invoice does not belong to you".to_string(),
                    }),
                ));
            }

            Ok(Json(json!({
                "success": true,
                "invoice": invoice,
                "message": "Invoice synced successfully",
            })))
        },
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "NotFound".to_string(),
                message: "Invoice not found".to_string(),
            }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "SyncFailed".to_string(),
                message: format!("Failed to sync invoice: {}", e),
            }),
        )),
    }
}

// ============================================================================
// Payment Endpoints (Stripe Integration)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCustomerRequest {
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachPaymentMethodRequest {
    pub payment_method_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub tier: String,
}

/// Create Stripe customer for authenticated user
pub async fn create_customer(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    // Parse request body
    let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Failed to read request body: {}", e),
            }),
        ))?;
    
    let req: CreateCustomerRequest = serde_json::from_slice(&bytes)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Invalid JSON: {}", e),
            }),
        ))?;
    
    if let Some(payment_mgr) = state.payment_manager() {
        let email = req.email.as_deref().ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: "Email is required".to_string(),
            }),
        ))?;
        
        match payment_mgr.create_customer(&api_key, email, req.name.as_deref()).await {
            Ok(customer_id) => {
                // Get customer info from database
                let customer = payment_mgr.get_customer(&api_key).await.map_err(|e| (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "DatabaseError".to_string(),
                        message: format!("Failed to retrieve customer: {}", e),
                    }),
                ))?;
                
                Ok(Json(serde_json::json!({
                    "success": true,
                    "customer_id": customer_id,
                    "created_at": customer.map(|c| c.created_at).unwrap_or(0),
                })))
            },
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "CustomerCreationFailed".to_string(),
                    message: format!("Failed to create customer: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Get customer payment info
pub async fn get_customer_info(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    if let Some(payment_mgr) = state.payment_manager() {
        match payment_mgr.get_customer(&api_key).await {
            Ok(Some(customer)) => Ok(Json(serde_json::json!({
                "customer_id": customer.stripe_customer_id,
                "subscription_id": customer.stripe_subscription_id,
                "payment_method_id": customer.payment_method_id,
                "subscription_status": customer.subscription_status,
                "created_at": customer.created_at,
                "updated_at": customer.updated_at,
            }))),
            Ok(None) => Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "CustomerNotFound".to_string(),
                    message: "No customer record found. Please create a customer first.".to_string(),
                }),
            )),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "DatabaseError".to_string(),
                    message: format!("Failed to retrieve customer: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Attach payment method to customer
pub async fn attach_payment_method(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    // Parse request body
    let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Failed to read request body: {}", e),
            }),
        ))?;
    
    let req: AttachPaymentMethodRequest = serde_json::from_slice(&bytes)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Invalid JSON: {}", e),
            }),
        ))?;
    
    if let Some(payment_mgr) = state.payment_manager() {
        match payment_mgr.attach_payment_method(&api_key, &req.payment_method_id).await {
            Ok(_) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "Payment method attached successfully",
                "payment_method_id": req.payment_method_id,
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "PaymentMethodFailed".to_string(),
                    message: format!("Failed to attach payment method: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Create subscription for a tier (requires payment method)
pub async fn create_subscription(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    // Parse request body
    let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Failed to read request body: {}", e),
            }),
        ))?;
    
    let req: CreateSubscriptionRequest = serde_json::from_slice(&bytes)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Invalid JSON: {}", e),
            }),
        ))?;
    
    if let Some(payment_mgr) = state.payment_manager() {
        if let Some(tier_mgr) = state.tier_manager() {
            // Parse tier level
            use nanolambda_storage::tier::TierLevel;
            let tier = match req.tier.to_lowercase().as_str() {
                "starter" => TierLevel::Starter,
                "pro" => TierLevel::Pro,
                "enterprise" => TierLevel::Enterprise,
                _ => return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "InvalidTier".to_string(),
                        message: "Invalid tier. Must be 'starter', 'pro', or 'enterprise'".to_string(),
                    }),
                )),
            };
            
            // Get Stripe price IDs from environment
            use nanolambda_storage::payment::StripePriceIds;
            let price_ids = StripePriceIds::from_env()
                .map_err(|e| (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "ConfigError".to_string(),
                        message: format!("Failed to load price configuration: {}", e),
                    }),
                ))?;
            
            // Create subscription
            match payment_mgr.create_subscription(&api_key, &tier, &price_ids).await {
                Ok(subscription) => {
                    // Assign tier to user
                    if let Err(e) = tier_mgr.assign_tier(&api_key, tier).await {
                        error!("Failed to assign tier after subscription: {}", e);
                    }
                    
                    Ok(Json(serde_json::json!({
                        "success": true,
                        "subscription_id": subscription.id.to_string(),
                        "status": subscription.status.to_string(),
                        "tier": req.tier,
                    })))
                }
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "SubscriptionFailed".to_string(),
                        message: format!("Failed to create subscription: {}", e),
                    }),
                ))
            }
        } else {
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "TierUnavailable".to_string(),
                    message: "Tier management not available".to_string(),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Cancel subscription
pub async fn cancel_subscription(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    if let Some(payment_mgr) = state.payment_manager() {
        match payment_mgr.cancel_subscription(&api_key).await {
            Ok(subscription) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "Subscription canceled successfully",
                "subscription_id": subscription.id.to_string(),
                "status": subscription.status.to_string(),
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "CancellationFailed".to_string(),
                    message: format!("Failed to cancel subscription: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Stripe webhook handler
pub async fn stripe_webhook(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(payment_mgr) = state.payment_manager() {
        // Get Stripe signature from headers
        let signature = headers
            .get("stripe-signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "MissingSignature".to_string(),
                    message: "Missing Stripe signature header".to_string(),
                }),
            ))?;
        
        // Get webhook secret from environment
        let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .map_err(|_| (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "ConfigError".to_string(),
                    message: "Stripe webhook secret not configured".to_string(),
                }),
            ))?;
        
        // Handle webhook event
        match payment_mgr.handle_webhook(&body, signature, &webhook_secret).await {
            Ok(_) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "Webhook processed successfully",
            }))),
            Err(e) => {
                error!("Webhook processing failed: {}", e);
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "WebhookFailed".to_string(),
                        message: format!("Failed to process webhook: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Report metered usage to Stripe
pub async fn report_metered_usage(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    #[derive(Deserialize)]
    struct ReportUsageRequest {
        quantity: i64,
        #[serde(default)]
        timestamp: Option<i64>,
    }
    
    let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Failed to read request body: {}", e),
            }),
        ))?;
    
    let req: ReportUsageRequest = serde_json::from_slice(&bytes)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidRequest".to_string(),
                message: format!("Invalid JSON: {}", e),
            }),
        ))?;
    
    if let Some(payment_mgr) = state.payment_manager() {
        match payment_mgr.report_usage(&api_key, req.quantity, req.timestamp).await {
            Ok(usage_record) => Ok(Json(serde_json::json!({
                "success": true,
                "usage_record_id": usage_record.id,
                "quantity": usage_record.quantity,
                "timestamp": usage_record.timestamp,
            }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "UsageReportFailed".to_string(),
                    message: format!("Failed to report usage: {}", e),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Calculate overage costs
pub async fn calculate_overage(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();
    let api_key = auth_ctx.as_ref().map(|ctx| ctx.api_key.clone()).unwrap_or_default();
    
    if api_key.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing API key".to_string(),
            }),
        ));
    }
    
    if let Some(payment_mgr) = state.payment_manager() {
        // Get current tier info
        if let Some(tier_mgr) = state.tier_manager() {
            match tier_mgr.get_user_tier(&api_key).await {
                Ok(user_tier) => {
                    // Get tier config to find limits
                    let tier_config = tier_mgr.get_tier_config(user_tier.tier).await;
                    let tier_limit = tier_config.max_invocations_per_month;
                    let current_usage = user_tier.monthly_invocations;
                    
                    match payment_mgr.calculate_overage_cost(&api_key, current_usage, tier_limit).await {
                        Ok(cost) => {
                            let overage = if let Some(limit) = tier_limit {
                                std::cmp::max(0, current_usage - limit)
                            } else {
                                0
                            };
                            
                            Ok(Json(serde_json::json!({
                                "success": true,
                                "current_usage": current_usage,
                                "tier_limit": tier_limit,
                                "overage_invocations": overage,
                                "overage_cost": cost,
                                "currency": "usd"
                            })))
                        },
                        Err(e) => Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: "CalculationFailed".to_string(),
                                message: format!("Failed to calculate overage: {}", e),
                            }),
                        ))
                    }
                },
                    Err(_) => {
                        // No tier assigned, use trial limits or return 0
                        Ok(Json(serde_json::json!({
                            "success": true,
                            "current_usage": 0,
                            "tier_limit": null,
                            "overage_invocations": 0,
                            "overage_cost": 0.0,
                            "currency": "usd",
                            "note": "No tier assigned. Currently in trial period."
                    })))
                }
            }
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "TierManagerUnavailable".to_string(),
                    message: "Tier manager not available".to_string(),
                }),
            ))
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}
/// Create a Stripe Customer Portal session
/// This returns a URL to redirect the customer to manage their subscription
pub async fn create_portal_session(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();

    let api_key = auth_ctx
        .map(|ctx| ctx.api_key)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(payment_mgr) = state.payment_manager() {
        // Parse optional return URL from request body
        let body_bytes = axum::body::to_bytes(
            req.into_body(),
            usize::MAX,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidBody".to_string(),
                    message: format!("Failed to read request body: {}", e),
                }),
            )
        })?;

        let return_url = if !body_bytes.is_empty() {
            #[derive(Deserialize)]
            struct PortalRequest {
                return_url: Option<String>,
            }

            let portal_req: PortalRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "InvalidJSON".to_string(),
                        message: format!("Failed to parse JSON: {}", e),
                    }),
                )
            })?;

            portal_req.return_url
        } else {
            None
        };

        match payment_mgr
            .create_portal_session(&api_key, return_url.as_deref())
            .await
        {
            Ok(session) => Ok(Json(serde_json::json!({
                "success": true,
                "portal_url": session.url,
                "session_id": session.id,
            }))),
            Err(e) => {
                error!("Failed to create portal session: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "PortalSessionFailed".to_string(),
                        message: format!("Failed to create portal session: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available (Stripe not configured)".to_string(),
            }),
        ))
    }
}

/// Check usage and send alerts if thresholds are reached
/// POST /usage/check-alerts
pub async fn check_usage_alerts(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();

    let api_key = auth_ctx
        .map(|ctx| ctx.api_key)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(payment_mgr) = state.payment_manager() {
        match payment_mgr.check_usage_alerts(&api_key).await {
            Ok(alerts) => {
                let alert_count = alerts.len();
                Ok(Json(serde_json::json!({
                    "success": true,
                    "alerts_sent": alert_count,
                    "alerts": alerts.iter().map(|a| {
                        serde_json::json!({
                            "type": match a.alert_type {
                                nanolambda_storage::payment::UsageAlertType::Warning80 => "warning80",
                                nanolambda_storage::payment::UsageAlertType::Warning90 => "warning90",
                                nanolambda_storage::payment::UsageAlertType::Critical100 => "critical100",
                            },
                            "threshold_percent": a.threshold_percent,
                            "current_usage": a.current_usage,
                            "usage_limit": a.usage_limit,
                            "sent_at": a.sent_at,
                        })
                    }).collect::<Vec<_>>(),
                })))
            }
            Err(e) => {
                error!("Failed to check usage alerts: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "AlertCheckFailed".to_string(),
                        message: format!("Failed to check usage alerts: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available".to_string(),
            }),
        ))
    }
}

/// Get usage alert history
/// GET /usage/alerts
pub async fn get_usage_alerts(
    State(state): State<Arc<ApiServer>>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let auth_ctx = req.extensions().get::<crate::auth::AuthContext>().cloned();

    let api_key = auth_ctx
        .map(|ctx| ctx.api_key)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(payment_mgr) = state.payment_manager() {
        match payment_mgr.get_usage_alerts(&api_key).await {
            Ok(alerts) => Ok(Json(serde_json::json!({
                "success": true,
                "alerts": alerts.iter().map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "type": match a.alert_type {
                            nanolambda_storage::payment::UsageAlertType::Warning80 => "warning80",
                            nanolambda_storage::payment::UsageAlertType::Warning90 => "warning90",
                            nanolambda_storage::payment::UsageAlertType::Critical100 => "critical100",
                        },
                        "threshold_percent": a.threshold_percent,
                        "current_usage": a.current_usage,
                        "usage_limit": a.usage_limit,
                        "sent_at": a.sent_at,
                        "period_start": a.period_start,
                        "period_end": a.period_end,
                    })
                }).collect::<Vec<_>>(),
            }))),
            Err(e) => {
                error!("Failed to get usage alerts: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "AlertFetchFailed".to_string(),
                        message: format!("Failed to fetch usage alerts: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "PaymentUnavailable".to_string(),
                message: "Payment processing not available".to_string(),
            }),
        ))
    }
}