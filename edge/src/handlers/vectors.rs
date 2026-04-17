//! Vector operations handler (QuartzDB integration)

use worker::{Env, Request, Response};

use crate::error::EdgeError;
use crate::types::{
    AuthContext, Permission, VectorDeleteRequest, VectorQueryRequest, VectorUpsertRequest,
};

/// Upsert vectors into the index
pub async fn upsert(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:write permission".to_string(),
        ));
    }

    let body: VectorUpsertRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate vectors
    if body.vectors.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "At least one vector is required".to_string(),
        ));
    }

    // Validate dimensions consistency
    let dimensions = body.vectors[0].values.len();
    for (i, v) in body.vectors.iter().enumerate() {
        if v.values.len() != dimensions {
            return Err(EdgeError::InvalidRequest(format!(
                "Vector {} has {} dimensions, expected {}",
                i,
                v.values.len(),
                dimensions
            )));
        }
    }

    // Get the VectorIndex Durable Object
    let namespace = env
        .durable_object("VECTOR_INDEX")
        .map_err(|e| EdgeError::Internal(format!("Durable Object binding error: {}", e)))?;

    // Use user_id + namespace as the DO id for isolation
    let ns = if body.namespace.is_empty() {
        "default"
    } else {
        &body.namespace
    };
    let do_id = namespace.id_from_name(&format!("{}:{}", auth.user_id, ns))?;
    let stub = do_id.get_stub()?;

    // Forward request to Durable Object
    let do_request = Request::new_with_init(
        "https://internal/upsert",
        worker::RequestInit::new()
            .with_method(worker::Method::Post)
            .with_body(Some(serde_json::to_string(&body)?.into())),
    )?;

    let response = stub.fetch_with_request(do_request).await?;

    // Return the DO response
    Ok(response)
}

/// Query vectors from the index
pub async fn query(mut req: Request, env: &Env, auth: &AuthContext) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:read permission".to_string(),
        ));
    }

    let body: VectorQueryRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate query vector
    if body.vector.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Query vector cannot be empty".to_string(),
        ));
    }

    // Validate top_k
    if body.top_k == 0 || body.top_k > 10000 {
        return Err(EdgeError::InvalidRequest(
            "top_k must be between 1 and 10000".to_string(),
        ));
    }

    // Get the VectorIndex Durable Object
    let namespace = env
        .durable_object("VECTOR_INDEX")
        .map_err(|e| EdgeError::Internal(format!("Durable Object binding error: {e}")))?;

    let ns = if body.namespace.is_empty() {
        "default"
    } else {
        &body.namespace
    };
    let do_id = namespace.id_from_name(&format!("{}:{}", auth.user_id, ns))?;
    let stub = do_id.get_stub()?;

    // Forward request to Durable Object
    let do_request = Request::new_with_init(
        "https://internal/query",
        worker::RequestInit::new()
            .with_method(worker::Method::Post)
            .with_body(Some(serde_json::to_string(&body)?.into())),
    )?;

    let response = stub.fetch_with_request(do_request).await?;

    Ok(response)
}

/// Delete vectors from the index
pub async fn delete(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:write permission".to_string(),
        ));
    }

    let body: VectorDeleteRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate delete request
    if body.ids.is_none() && !body.delete_all && body.filter.is_none() {
        return Err(EdgeError::InvalidRequest(
            "Must specify ids, filter, or delete_all".to_string(),
        ));
    }

    // Get the VectorIndex Durable Object
    let namespace = env
        .durable_object("VECTOR_INDEX")
        .map_err(|e| EdgeError::Internal(format!("Durable Object binding error: {e}")))?;

    let ns = if body.namespace.is_empty() {
        "default"
    } else {
        &body.namespace
    };
    let do_id = namespace.id_from_name(&format!("{}:{}", auth.user_id, ns))?;
    let stub = do_id.get_stub()?;

    // Forward request to Durable Object
    let do_request = Request::new_with_init(
        "https://internal/delete",
        worker::RequestInit::new()
            .with_method(worker::Method::Post)
            .with_body(Some(serde_json::to_string(&body)?.into())),
    )?;

    let response = stub.fetch_with_request(do_request).await?;

    Ok(response)
}

/// Get vector index statistics
pub async fn stats(env: &Env, auth: &AuthContext) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:read permission".to_string(),
        ));
    }

    // Get the VectorIndex Durable Object for default namespace
    let namespace = env
        .durable_object("VECTOR_INDEX")
        .map_err(|e| EdgeError::Internal(format!("Durable Object binding error: {e}")))?;

    let do_id = namespace.id_from_name(&format!("{}:default", auth.user_id))?;
    let stub = do_id.get_stub()?;

    // Forward request to Durable Object
    let do_request = Request::new("https://internal/stats", worker::Method::Get)?;
    let response = stub.fetch_with_request(do_request).await?;

    Ok(response)
}

/// List all namespaces
pub async fn list_namespaces(_env: &Env, auth: &AuthContext) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:read permission".to_string(),
        ));
    }

    // Note: Listing all DOs for a user requires a separate tracking mechanism
    // For now, return a placeholder
    Response::from_json(&serde_json::json!({
        "namespaces": ["default"],
        "note": "Full namespace listing requires additional index implementation"
    }))
    .map_err(EdgeError::from)
}
