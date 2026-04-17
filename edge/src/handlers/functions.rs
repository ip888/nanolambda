//! Function CRUD handlers

use chrono::Utc;
use uuid::Uuid;
use worker::{Env, Request, Response};

use crate::error::EdgeError;
use crate::types::{
    AuthContext, CreateFunctionRequest, Function, Permission, Runtime, UpdateFunctionRequest,
};

/// List all functions for the authenticated user
pub async fn list_functions(env: &Env, auth: &AuthContext) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::FunctionsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing functions:read permission".to_string(),
        ));
    }

    let kv = env
        .kv("FUNCTIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {}", e)))?;

    // List all functions for this user
    // Note: In production, we'd use a secondary index or prefix-based listing
    let list_result = kv
        .list()
        .prefix(format!("user:{}:", auth.user_id))
        .execute()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to list functions: {e}")))?;

    let mut functions = Vec::new();
    for key in list_result.keys {
        if let Some(func) = kv
            .get(&key.name)
            .json::<Function>()
            .await
            .map_err(|e| EdgeError::StorageError(e.to_string()))?
        {
            functions.push(func);
        }
    }

    Response::from_json(&serde_json::json!({
        "functions": functions,
        "total": functions.len()
    }))
    .map_err(EdgeError::from)
}

/// Create a new function
pub async fn create_function(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::FunctionsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing functions:write permission".to_string(),
        ));
    }

    let body: CreateFunctionRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate function name
    if body.name.is_empty() || body.name.len() > 64 {
        return Err(EdgeError::InvalidRequest(
            "Function name must be 1-64 characters".to_string(),
        ));
    }

    // Validate runtime
    if body.runtime == Runtime::Python {
        // Python support via Pyodide - future feature
        return Err(EdgeError::NotImplemented(
            "Python runtime coming soon. Use JavaScript for now.".to_string(),
        ));
    }

    // Validate code is not empty
    if body.code.trim().is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Function code cannot be empty".to_string(),
        ));
    }

    // Validate timeout
    if body.timeout_ms > 300_000 {
        return Err(EdgeError::InvalidRequest(
            "Timeout cannot exceed 300 seconds".to_string(),
        ));
    }

    // Validate memory
    if body.memory_mb < 128 || body.memory_mb > 1024 {
        return Err(EdgeError::InvalidRequest(
            "Memory must be between 128MB and 1024MB".to_string(),
        ));
    }

    let now = Utc::now();
    let function_id = Uuid::new_v4().to_string();

    let function = Function {
        id: function_id.clone(),
        name: body.name,
        runtime: body.runtime,
        code: body.code,
        handler: body.handler,
        timeout_ms: body.timeout_ms,
        memory_mb: body.memory_mb,
        environment: body.environment,
        created_at: now,
        updated_at: now,
        version: 1,
        owner_id: auth.user_id.clone(),
    };

    let kv = env
        .kv("FUNCTIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    // Store with user prefix for listing
    let key = format!("user:{}:{}", auth.user_id, function_id);
    kv.put(&key, &function)
        .map_err(|e| EdgeError::StorageError(e.to_string()))?
        .execute()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to store function: {e}")))?;

    // Also store by ID for direct lookup
    kv.put(&format!("id:{}", function_id), &function)
        .map_err(|e| EdgeError::StorageError(e.to_string()))?
        .execute()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to store function: {e}")))?;

    Response::from_json(&serde_json::json!({
        "function": function,
        "message": "Function created successfully"
    }))
    .map(|r| r.with_status(201))
    .map_err(EdgeError::from)
}

/// Get a function by ID
pub async fn get_function(id: &str, env: &Env, auth: &AuthContext) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::FunctionsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing functions:read permission".to_string(),
        ));
    }

    let function = fetch_function(id, env).await?;

    // Check ownership
    if function.owner_id != auth.user_id && !auth.has_permission(&Permission::Admin) {
        return Err(EdgeError::AuthorizationDenied(
            "You don't have access to this function".to_string(),
        ));
    }

    Response::from_json(&function).map_err(EdgeError::from)
}

/// Update a function
pub async fn update_function(
    id: &str,
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::FunctionsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing functions:write permission".to_string(),
        ));
    }

    let body: UpdateFunctionRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    let mut function = fetch_function(id, env).await?;

    // Check ownership
    if function.owner_id != auth.user_id && !auth.has_permission(&Permission::Admin) {
        return Err(EdgeError::AuthorizationDenied(
            "You don't have access to this function".to_string(),
        ));
    }

    // Apply updates
    if let Some(code) = body.code {
        if code.trim().is_empty() {
            return Err(EdgeError::InvalidRequest(
                "Function code cannot be empty".to_string(),
            ));
        }
        function.code = code;
    }
    if let Some(handler) = body.handler {
        function.handler = handler;
    }
    if let Some(timeout_ms) = body.timeout_ms {
        if timeout_ms > 300_000 {
            return Err(EdgeError::InvalidRequest(
                "Timeout cannot exceed 300 seconds".to_string(),
            ));
        }
        function.timeout_ms = timeout_ms;
    }
    if let Some(memory_mb) = body.memory_mb {
        if memory_mb < 128 || memory_mb > 1024 {
            return Err(EdgeError::InvalidRequest(
                "Memory must be between 128MB and 1024MB".to_string(),
            ));
        }
        function.memory_mb = memory_mb;
    }
    if let Some(environment) = body.environment {
        function.environment = environment;
    }

    function.updated_at = Utc::now();
    function.version += 1;

    let kv = env
        .kv("FUNCTIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    // Update both keys
    let user_key = format!("user:{}:{}", auth.user_id, id);
    let id_key = format!("id:{}", id);

    kv.put(&user_key, &function)
        .map_err(|e| EdgeError::StorageError(e.to_string()))?
        .execute()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to update function: {e}")))?;

    kv.put(&id_key, &function)
        .map_err(|e| EdgeError::StorageError(e.to_string()))?
        .execute()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to update function: {e}")))?;

    Response::from_json(&serde_json::json!({
        "function": function,
        "message": "Function updated successfully"
    }))
    .map_err(EdgeError::from)
}

/// Delete a function
pub async fn delete_function(
    id: &str,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::FunctionsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing functions:write permission".to_string(),
        ));
    }

    let function = fetch_function(id, env).await?;

    // Check ownership
    if function.owner_id != auth.user_id && !auth.has_permission(&Permission::Admin) {
        return Err(EdgeError::AuthorizationDenied(
            "You don't have access to this function".to_string(),
        ));
    }

    let kv = env
        .kv("FUNCTIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    // Delete both keys
    let user_key = format!("user:{}:{}", auth.user_id, id);
    let id_key = format!("id:{}", id);

    kv.delete(&user_key)
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to delete function: {e}")))?;

    kv.delete(&id_key)
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to delete function: {e}")))?;

    Response::from_json(&serde_json::json!({
        "message": "Function deleted successfully",
        "id": id
    }))
    .map_err(EdgeError::from)
}

/// Helper to fetch a function by ID
pub async fn fetch_function(id: &str, env: &Env) -> Result<Function, EdgeError> {
    let kv = env
        .kv("FUNCTIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    let key = format!("id:{}", id);
    let function: Option<Function> = kv
        .get(&key)
        .json()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to fetch function: {e}")))?;

    function.ok_or_else(|| EdgeError::FunctionNotFound(id.to_string()))
}
