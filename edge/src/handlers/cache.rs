//! Semantic Cache API Handler
//!
//! Provides HTTP endpoints for semantic caching operations:
//! - POST /cache/query - Check cache for similar query
//! - POST /cache/store - Store query/response pair
//! - DELETE /cache/clear - Clear cache
//! - GET /cache/stats - Get cache statistics

use serde::{Deserialize, Serialize};
use worker::{Env, Request, Response};
use std::collections::HashMap;

use crate::cache::{CacheConfig, CacheEntry, SemanticCache};
use crate::error::EdgeError;
use crate::types::{AuthContext, Permission};

/// Request to query the semantic cache
#[derive(Debug, Deserialize)]
pub struct CacheQueryRequest {
    /// Query embedding vector
    pub embedding: Vec<f32>,
    /// Namespace (optional)
    #[serde(default)]
    pub namespace: String,
    /// Similarity threshold override (optional)
    pub similarity_threshold: Option<f32>,
}

/// Response from cache query
#[derive(Debug, Serialize)]
pub struct CacheQueryResponse {
    /// Whether cache hit occurred
    pub hit: bool,
    /// Cached response (if hit)
    pub response: Option<serde_json::Value>,
    /// Similarity score (if hit)
    pub similarity: Option<f32>,
    /// Cache entry ID (if hit)
    pub entry_id: Option<String>,
    /// Estimated latency saved in ms
    pub latency_saved_ms: Option<u64>,
}

/// Request to store in semantic cache
#[derive(Debug, Deserialize)]
pub struct CacheStoreRequest {
    /// Query text (for reference)
    pub query: String,
    /// Query embedding vector
    pub embedding: Vec<f32>,
    /// Response to cache
    pub response: serde_json::Value,
    /// Model that generated the response
    pub model: String,
    /// Namespace (optional)
    #[serde(default)]
    pub namespace: String,
    /// Custom metadata (optional)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Response from cache store
#[derive(Debug, Serialize)]
pub struct CacheStoreResponse {
    /// Success indicator
    pub success: bool,
    /// Entry ID
    pub entry_id: String,
    /// Namespace stored to
    pub namespace: String,
}

/// Cache statistics response
#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    /// Total queries
    pub total_queries: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate percentage
    pub hit_rate: f32,
    /// Number of entries
    pub entry_count: usize,
    /// Approximate bytes used
    pub bytes_used: usize,
    /// Average similarity on hits
    pub avg_hit_similarity: f32,
    /// Cache configuration
    pub config: CacheConfig,
}

/// Query the semantic cache
pub async fn query_cache(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AIRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:read permission".to_string(),
        ));
    }

    let body: CacheQueryRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate embedding
    if body.embedding.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Embedding vector cannot be empty".to_string(),
        ));
    }

    // Get or create cache from KV
    let namespace = if body.namespace.is_empty() { "default" } else { &body.namespace };
    let cache_key = format!("cache:{}:{}", auth.user_id, namespace);
    
    let kv = env.kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;
    
    let mut cache = match kv.get(&cache_key).text().await
        .map_err(|e| EdgeError::Internal(format!("KV get error: {e}")))? 
    {
        Some(json) => SemanticCache::from_json(&json)
            .map_err(|e| EdgeError::Internal(format!("Cache deserialize error: {e}")))?,
        None => SemanticCache::new(namespace.to_string()),
    };

    // Apply threshold override if provided
    if let Some(threshold) = body.similarity_threshold {
        cache.config.similarity_threshold = threshold.clamp(0.0, 1.0);
    }

    // Query cache
    let result = cache.query(&body.embedding);

    // Save updated cache (for hit count tracking)
    let cache_json = cache.to_json()
        .map_err(|e| EdgeError::Internal(format!("Cache serialize error: {e}")))?;
    kv.put(&cache_key, &cache_json)
        .map_err(|e| EdgeError::Internal(format!("KV put error: {e}")))?
        .execute()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV put execute error: {e}")))?;

    let response = match result {
        Some(hit) => CacheQueryResponse {
            hit: true,
            response: Some(hit.entry.response),
            similarity: Some(hit.similarity),
            entry_id: Some(hit.entry.id),
            latency_saved_ms: Some(hit.latency_saved_ms),
        },
        None => CacheQueryResponse {
            hit: false,
            response: None,
            similarity: None,
            entry_id: None,
            latency_saved_ms: None,
        },
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Store in semantic cache
pub async fn store_cache(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AIWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:write permission".to_string(),
        ));
    }

    let body: CacheStoreRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate
    if body.embedding.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Embedding vector cannot be empty".to_string(),
        ));
    }
    if body.query.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Query text cannot be empty".to_string(),
        ));
    }

    // Get or create cache from KV
    let namespace = if body.namespace.is_empty() { "default" } else { &body.namespace };
    let cache_key = format!("cache:{}:{}", auth.user_id, namespace);
    
    let kv = env.kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;
    
    let mut cache = match kv.get(&cache_key).text().await
        .map_err(|e| EdgeError::Internal(format!("KV get error: {e}")))? 
    {
        Some(json) => SemanticCache::from_json(&json)
            .map_err(|e| EdgeError::Internal(format!("Cache deserialize error: {e}")))?,
        None => SemanticCache::new(namespace.to_string()),
    };

    // Generate entry ID
    let entry_id = format!("entry_{}", uuid_v4());

    // Create entry
    let mut entry = CacheEntry::new(
        entry_id.clone(),
        body.query,
        body.embedding,
        body.response,
        body.model,
    );
    entry.metadata = body.metadata;

    // Insert into cache
    cache.insert(entry);

    // Save cache
    let cache_json = cache.to_json()
        .map_err(|e| EdgeError::Internal(format!("Cache serialize error: {e}")))?;
    kv.put(&cache_key, &cache_json)
        .map_err(|e| EdgeError::Internal(format!("KV put error: {e}")))?
        .execute()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV put execute error: {e}")))?;

    let response = CacheStoreResponse {
        success: true,
        entry_id,
        namespace: namespace.to_string(),
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Clear semantic cache
pub async fn clear_cache(
    env: &Env,
    auth: &AuthContext,
    namespace: Option<&str>,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AIWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:write permission".to_string(),
        ));
    }

    let namespace = namespace.unwrap_or("default");
    let cache_key = format!("cache:{}:{}", auth.user_id, namespace);
    
    let kv = env.kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;
    
    // Delete the cache
    kv.delete(&cache_key)
        .await
        .map_err(|e| EdgeError::Internal(format!("KV delete error: {e}")))?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "namespace": namespace,
        "message": "Cache cleared"
    })).map_err(EdgeError::from)
}

/// Get cache statistics
pub async fn cache_stats(
    env: &Env,
    auth: &AuthContext,
    namespace: Option<&str>,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AIRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:read permission".to_string(),
        ));
    }

    let namespace = namespace.unwrap_or("default");
    let cache_key = format!("cache:{}:{}", auth.user_id, namespace);
    
    let kv = env.kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;
    
    let cache = match kv.get(&cache_key).text().await
        .map_err(|e| EdgeError::Internal(format!("KV get error: {e}")))? 
    {
        Some(json) => SemanticCache::from_json(&json)
            .map_err(|e| EdgeError::Internal(format!("Cache deserialize error: {e}")))?,
        None => SemanticCache::new(namespace.to_string()),
    };

    let stats = cache.stats();
    let response = CacheStatsResponse {
        total_queries: stats.total_queries,
        hits: stats.hits,
        misses: stats.misses,
        hit_rate: stats.hit_rate(),
        entry_count: stats.entry_count,
        bytes_used: stats.bytes_used,
        avg_hit_similarity: stats.avg_hit_similarity,
        config: cache.config.clone(),
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Generate a UUID v4 (simple implementation for WASM)
fn uuid_v4() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    
    let bytes: [u8; 16] = rng.random();
    
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        (bytes[6] & 0x0f) | 0x40, bytes[7],
        (bytes[8] & 0x3f) | 0x80, bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}
