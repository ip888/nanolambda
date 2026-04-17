//! Hybrid Search API Handler
//!
//! Provides HTTP endpoints for hybrid search (BM25 + Vector):
//! - POST /search/index - Index documents for BM25
//! - POST /search/hybrid - Perform hybrid search
//! - POST /search/rerank - Rerank search results
//! - DELETE /search/index - Remove documents from index

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::{Env, Request, Response};

use crate::error::EdgeError;
use crate::search::{
    FusionMethod, HybridResult, HybridSearch, RankedResult, Reranker, RerankerConfig,
    RerankerDocument, TokenizedDocument, VectorResult,
};
use crate::types::{AuthContext, Permission};

/// Request to index documents for BM25
#[derive(Debug, Deserialize)]
pub struct IndexDocumentsRequest {
    /// Documents to index
    pub documents: Vec<DocumentInput>,
    /// Namespace (optional)
    #[serde(default)]
    pub namespace: String,
}

/// Document input for indexing
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentInput {
    /// Document ID
    pub id: String,
    /// Document text content
    pub text: String,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Response from indexing
#[derive(Debug, Serialize)]
pub struct IndexDocumentsResponse {
    pub success: bool,
    pub indexed_count: usize,
    pub namespace: String,
    pub total_documents: usize,
    pub vocabulary_size: usize,
}

/// Request for hybrid search
#[derive(Debug, Deserialize)]
pub struct HybridSearchRequest {
    /// Query text
    pub query: String,
    /// Query embedding (for dense search)
    /// Reserved for future dense vector fusion support
    #[allow(dead_code)]
    pub query_embedding: Vec<f32>,
    /// Vector search results (from QuartzDB)
    #[serde(default)]
    pub vector_results: Vec<VectorResultInput>,
    /// Number of results to return
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Namespace
    #[serde(default)]
    pub namespace: String,
    /// Sparse weight (0.0-1.0)
    pub sparse_weight: Option<f32>,
    /// Dense weight (0.0-1.0)
    pub dense_weight: Option<f32>,
    /// Fusion method
    pub fusion_method: Option<String>,
}

/// Vector result input (from external vector search)
#[derive(Debug, Clone, Deserialize)]
pub struct VectorResultInput {
    pub id: String,
    pub score: f32,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_top_k() -> usize {
    10
}

/// Response from hybrid search
#[derive(Debug, Serialize)]
pub struct HybridSearchResponse {
    pub results: Vec<HybridResult>,
    pub query: String,
    pub total_results: usize,
}

/// Request for reranking
#[derive(Debug, Deserialize)]
pub struct RerankRequest {
    /// Query text
    pub query: String,
    /// Query embedding (optional, for semantic scoring)
    pub query_embedding: Option<Vec<f32>>,
    /// Documents to rerank
    pub documents: Vec<RerankDocumentInput>,
    /// Number of results to return
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Include document text in response
    #[serde(default)]
    pub include_text: bool,
    /// Reranker configuration overrides
    pub config: Option<RerankerConfigInput>,
}

/// Document input for reranking
#[derive(Debug, Clone, Deserialize)]
pub struct RerankDocumentInput {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub initial_score: f32,
    pub embedding: Option<Vec<f32>>,
    pub timestamp: Option<String>,
    pub popularity: Option<u64>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Reranker config input
#[derive(Debug, Clone, Deserialize)]
pub struct RerankerConfigInput {
    pub semantic_weight: Option<f32>,
    pub keyword_weight: Option<f32>,
    pub recency_weight: Option<f32>,
    pub popularity_weight: Option<f32>,
    pub min_score: Option<f32>,
}

/// Response from reranking
#[derive(Debug, Serialize)]
pub struct RerankResponse {
    pub results: Vec<RankedResult>,
    pub query: String,
}

/// Index documents for BM25 search
pub async fn index_documents(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:write permission".to_string(),
        ));
    }

    let body: IndexDocumentsRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    if body.documents.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "At least one document is required".to_string(),
        ));
    }

    // Get or create BM25 index from KV
    let namespace = if body.namespace.is_empty() {
        "default"
    } else {
        &body.namespace
    };
    let index_key = format!("bm25:{}:{}", auth.user_id, namespace);

    let kv = env
        .kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    let mut hybrid = match kv
        .get(&index_key)
        .text()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV get error: {e}")))?
    {
        Some(json) => HybridSearch::from_json(&json)
            .map_err(|e| EdgeError::Internal(format!("Index deserialize error: {e}")))?,
        None => HybridSearch::new(),
    };

    let bm25_config = hybrid.bm25_index().config().clone();

    // Index documents
    let mut indexed = 0;
    for doc in body.documents {
        let tokenized =
            TokenizedDocument::with_metadata(doc.id, doc.text, doc.metadata, &bm25_config);
        hybrid.bm25_index_mut().add_document(tokenized);
        indexed += 1;
    }

    // Save index
    let index_json = hybrid
        .to_json()
        .map_err(|e| EdgeError::Internal(format!("Index serialize error: {e}")))?;
    kv.put(&index_key, &index_json)
        .map_err(|e| EdgeError::Internal(format!("KV put error: {e}")))?
        .execute()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV put execute error: {e}")))?;

    let response = IndexDocumentsResponse {
        success: true,
        indexed_count: indexed,
        namespace: namespace.to_string(),
        total_documents: hybrid.bm25_index().len(),
        vocabulary_size: hybrid.bm25_index().vocabulary_size(),
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Perform hybrid search
pub async fn hybrid_search(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:read permission".to_string(),
        ));
    }

    let body: HybridSearchRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    if body.query.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Query cannot be empty".to_string(),
        ));
    }

    // Get BM25 index from KV
    let namespace = if body.namespace.is_empty() {
        "default"
    } else {
        &body.namespace
    };
    let index_key = format!("bm25:{}:{}", auth.user_id, namespace);

    let kv = env
        .kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    let mut hybrid = match kv
        .get(&index_key)
        .text()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV get error: {e}")))?
    {
        Some(json) => HybridSearch::from_json(&json)
            .map_err(|e| EdgeError::Internal(format!("Index deserialize error: {e}")))?,
        None => HybridSearch::new(),
    };

    // Apply config overrides
    if let Some(sparse_w) = body.sparse_weight {
        hybrid.config.sparse_weight = sparse_w.clamp(0.0, 1.0);
    }
    if let Some(dense_w) = body.dense_weight {
        hybrid.config.dense_weight = dense_w.clamp(0.0, 1.0);
    }
    if let Some(method) = &body.fusion_method {
        hybrid.config.fusion_method = match method.to_lowercase().as_str() {
            "weighted" => FusionMethod::Weighted,
            "maxscore" | "max_score" => FusionMethod::MaxScore,
            _ => FusionMethod::RRF,
        };
    }

    // Convert vector results
    let vector_results: Vec<VectorResult> = body
        .vector_results
        .into_iter()
        .map(|r| VectorResult {
            id: r.id,
            score: r.score,
            metadata: r.metadata,
        })
        .collect();

    // Perform hybrid search
    let results = hybrid.search(&body.query, vector_results, body.top_k);

    let response = HybridSearchResponse {
        total_results: results.len(),
        results,
        query: body.query,
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Rerank search results
pub async fn rerank(
    mut req: Request,
    _env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsRead) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:read permission".to_string(),
        ));
    }

    let body: RerankRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    if body.query.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Query cannot be empty".to_string(),
        ));
    }
    if body.documents.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "At least one document is required".to_string(),
        ));
    }

    // Build reranker config
    let mut config = RerankerConfig::default();
    if let Some(cfg) = body.config {
        if let Some(w) = cfg.semantic_weight {
            config.semantic_weight = w;
        }
        if let Some(w) = cfg.keyword_weight {
            config.keyword_weight = w;
        }
        if let Some(w) = cfg.recency_weight {
            config.recency_weight = w;
        }
        if let Some(w) = cfg.popularity_weight {
            config.popularity_weight = w;
        }
        if let Some(m) = cfg.min_score {
            config.min_score = m;
        }
    }

    let reranker = Reranker::with_config(config);

    // Convert documents
    let documents: Vec<RerankerDocument> = body
        .documents
        .into_iter()
        .map(|d| {
            let mut doc = RerankerDocument::new(d.id, d.text, d.initial_score);
            if let Some(emb) = d.embedding {
                doc = doc.with_embedding(emb);
            }
            if let Some(ts) = d.timestamp {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&ts) {
                    doc = doc.with_timestamp(parsed.with_timezone(&chrono::Utc));
                }
            }
            if let Some(pop) = d.popularity {
                doc = doc.with_popularity(pop);
            }
            doc = doc.with_metadata(d.metadata);
            doc
        })
        .collect();

    // Perform reranking
    let query_embedding = body.query_embedding.as_deref();
    let results = reranker.rerank(
        &body.query,
        query_embedding,
        documents,
        body.top_k,
        body.include_text,
    );

    let response = RerankResponse {
        results,
        query: body.query,
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Remove documents from BM25 index
pub async fn remove_documents(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::VectorsWrite) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing vectors:write permission".to_string(),
        ));
    }

    #[derive(Deserialize)]
    struct RemoveRequest {
        ids: Vec<String>,
        #[serde(default)]
        namespace: String,
    }

    let body: RemoveRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    let namespace = if body.namespace.is_empty() {
        "default"
    } else {
        &body.namespace
    };
    let index_key = format!("bm25:{}:{}", auth.user_id, namespace);

    let kv = env
        .kv("SESSIONS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;

    let mut hybrid = match kv
        .get(&index_key)
        .text()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV get error: {e}")))?
    {
        Some(json) => HybridSearch::from_json(&json)
            .map_err(|e| EdgeError::Internal(format!("Index deserialize error: {e}")))?,
        None => return Err(EdgeError::NotFound("Index not found".to_string())),
    };

    let mut removed = 0;
    for id in &body.ids {
        if hybrid.bm25_index_mut().remove_document(id).is_some() {
            removed += 1;
        }
    }

    // Save updated index
    let index_json = hybrid
        .to_json()
        .map_err(|e| EdgeError::Internal(format!("Index serialize error: {e}")))?;
    kv.put(&index_key, &index_json)
        .map_err(|e| EdgeError::Internal(format!("KV put error: {e}")))?
        .execute()
        .await
        .map_err(|e| EdgeError::Internal(format!("KV put execute error: {e}")))?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "removed_count": removed,
        "namespace": namespace
    }))
    .map_err(EdgeError::from)
}
