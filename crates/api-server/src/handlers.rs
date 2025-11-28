//! Request handlers - Integrated with storage and runtime

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::Html,
};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
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
    
    // 0a. Check rate limit
    if let Err(e) = state.rate_limiter().check_rate_limit(&api_key).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "RateLimitExceeded".to_string(),
                message: format!("{}", e),
            }),
        ));
    }
    
    // 0b. Acquire concurrency permits (queues if at limit, rejects if queue full)
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
    
    // 2. Check if function is active
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
                // Calculate costs
                let invocation_cost = 0.00000016; // $0.16 per 1M invocations
                let gb_seconds = (exec_result.metrics.memory_peak_mb as f64 / 1024.0) 
                    * (exec_result.metrics.execution_ms as f64 / 1000.0);
                let compute_cost = gb_seconds * 0.000015; // $0.000015 per GB-second
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
