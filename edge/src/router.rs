//! Request router for NanoLambda Edge

use worker::*;
use crate::auth;
use crate::error::EdgeError;
use crate::handlers;
use crate::types::HealthResponse;
use chrono::Utc;

/// Main request router
pub async fn handle_request(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    let method = req.method();
    
    // CORS headers for all responses
    let cors_headers = |mut response: Response| -> Response {
        let headers = response.headers_mut();
        let _ = headers.set("Access-Control-Allow-Origin", "*");
        let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
        let _ = headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Api-Key");
        let _ = headers.set("Access-Control-Max-Age", "86400");
        response
    };
    
    // Handle CORS preflight
    if method == Method::Options {
        return Ok(cors_headers(Response::empty()?.with_status(204)));
    }
    
    // Route the request
    let result = route_request(req, &env, &ctx, &path, &method).await;
    
    // Convert result to response with CORS
    match result {
        Ok(response) => Ok(cors_headers(response)),
        Err(e) => Ok(cors_headers(e.to_response())),
    }
}

/// Route request to appropriate handler
async fn route_request(
    req: Request,
    env: &Env,
    ctx: &Context,
    path: &str,
    method: &Method,
) -> std::result::Result<Response, EdgeError> {
    // Health check - no auth required
    if path == "/health" || path == "/" {
        return handle_health(env).await;
    }
    
    // API version prefix
    let path = path.strip_prefix("/v1").unwrap_or(path);
    
    // Public endpoints (no auth)
    if path == "/health" {
        return handle_health(env).await;
    }
    
    // Authenticate for all other endpoints
    let auth_ctx = auth::authenticate(&req, env).await?;
    
    // Route based on path segments
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    match (method.clone(), segments.as_slice()) {
        // Functions CRUD
        (Method::Get, ["functions"]) => {
            handlers::functions::list_functions(env, &auth_ctx).await
        }
        (Method::Post, ["functions"]) => {
            handlers::functions::create_function(req, env, &auth_ctx).await
        }
        (Method::Get, ["functions", id]) => {
            handlers::functions::get_function(id, env, &auth_ctx).await
        }
        (Method::Put, ["functions", id]) => {
            handlers::functions::update_function(id, req, env, &auth_ctx).await
        }
        (Method::Delete, ["functions", id]) => {
            handlers::functions::delete_function(id, env, &auth_ctx).await
        }
        
        // Function invocation
        (Method::Post, ["functions", id, "invoke"]) => {
            handlers::invoke::invoke_function(id, req, env, ctx, &auth_ctx).await
        }
        (Method::Post, ["invoke", id]) => {
            // Alternative invoke path
            handlers::invoke::invoke_function(id, req, env, ctx, &auth_ctx).await
        }
        
        // Vectors (QuartzDB)
        (Method::Post, ["vectors", "upsert"]) => {
            handlers::vectors::upsert(req, env, &auth_ctx).await
        }
        (Method::Post, ["vectors", "query"]) => {
            handlers::vectors::query(req, env, &auth_ctx).await
        }
        (Method::Post, ["vectors", "delete"]) => {
            handlers::vectors::delete(req, env, &auth_ctx).await
        }
        (Method::Get, ["vectors", "stats"]) => {
            handlers::vectors::stats(env, &auth_ctx).await
        }
        (Method::Post, ["vectors", "namespaces"]) => {
            handlers::vectors::list_namespaces(env, &auth_ctx).await
        }
        
        // AI endpoints
        (Method::Post, ["ai", "embeddings"]) => {
            handlers::ai::embeddings(req, env, &auth_ctx).await
        }
        (Method::Post, ["ai", "completions"]) => {
            handlers::ai::completions(req, env, &auth_ctx).await
        }
        (Method::Post, ["ai", "chat"]) => {
            handlers::ai::chat(req, env, &auth_ctx).await
        }
        
        // Semantic Cache endpoints (UNIQUE FEATURE)
        (Method::Post, ["cache", "query"]) => {
            handlers::cache::query_cache(req, env, &auth_ctx).await
        }
        (Method::Post, ["cache", "store"]) => {
            handlers::cache::store_cache(req, env, &auth_ctx).await
        }
        (Method::Delete, ["cache"]) => {
            handlers::cache::clear_cache(env, &auth_ctx, None).await
        }
        (Method::Delete, ["cache", namespace]) => {
            handlers::cache::clear_cache(env, &auth_ctx, Some(namespace)).await
        }
        (Method::Get, ["cache", "stats"]) => {
            handlers::cache::cache_stats(env, &auth_ctx, None).await
        }
        (Method::Get, ["cache", "stats", namespace]) => {
            handlers::cache::cache_stats(env, &auth_ctx, Some(namespace)).await
        }
        
        // Hybrid Search endpoints (UNIQUE FEATURE)
        (Method::Post, ["search", "index"]) => {
            handlers::hybrid::index_documents(req, env, &auth_ctx).await
        }
        (Method::Post, ["search", "hybrid"]) => {
            handlers::hybrid::hybrid_search(req, env, &auth_ctx).await
        }
        (Method::Post, ["search", "rerank"]) => {
            handlers::hybrid::rerank(req, env, &auth_ctx).await
        }
        (Method::Delete, ["search", "index"]) => {
            handlers::hybrid::remove_documents(req, env, &auth_ctx).await
        }
        
        // Usage & billing
        (Method::Get, ["usage"]) => {
            handlers::usage::get_usage(req, env, &auth_ctx).await
        }
        (Method::Get, ["usage", "summary"]) => {
            handlers::usage::get_summary(env, &auth_ctx).await
        }
        
        // Not found
        _ => Err(EdgeError::InvalidRequest(format!(
            "Unknown endpoint: {} {}",
            method, path
        ))),
    }
}

/// Health check handler
async fn handle_health(env: &Env) -> std::result::Result<Response, EdgeError> {
    let region = env.var("CF_REGION")
        .map_or_else(|_| "unknown".to_string(), |v| v.to_string());
    
    let health = HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        region,
        timestamp: Utc::now(),
    };
    
    Response::from_json(&health).map_err(EdgeError::from)
}
