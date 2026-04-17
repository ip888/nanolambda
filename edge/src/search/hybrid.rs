//! Hybrid Search Implementation
//!
//! Combines sparse (BM25/keyword) and dense (vector) search using
//! Reciprocal Rank Fusion (RRF) or weighted combination.
//!
//! Why Hybrid Search?
//! - BM25 excels at exact keyword matching
//! - Vector search excels at semantic similarity
//! - Combining both gives superior retrieval quality
//!
//! Methods:
//! 1. RRF (Reciprocal Rank Fusion): Combines rankings without score normalization
//! 2. Weighted: Combines normalized scores with configurable weights

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::bm25::{Bm25Index, Bm25Result};

/// Fusion method for combining sparse and dense results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FusionMethod {
    /// Reciprocal Rank Fusion: 1/(k + rank)
    RRF,
    /// Weighted combination of normalized scores
    Weighted,
    /// Take best score from either method
    MaxScore,
}

impl Default for FusionMethod {
    fn default() -> Self {
        FusionMethod::RRF
    }
}

/// Hybrid search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// Weight for sparse (BM25) scores (0.0-1.0)
    pub sparse_weight: f32,
    /// Weight for dense (vector) scores (0.0-1.0)
    pub dense_weight: f32,
    /// Fusion method
    pub fusion_method: FusionMethod,
    /// RRF constant k (default: 60)
    pub rrf_k: u32,
    /// Minimum score threshold
    pub min_score: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            sparse_weight: 0.5,
            dense_weight: 0.5,
            fusion_method: FusionMethod::RRF,
            rrf_k: 60,
            min_score: 0.0,
        }
    }
}

impl HybridConfig {
    /// Create a config favoring keyword search
    pub fn keyword_focused() -> Self {
        Self {
            sparse_weight: 0.7,
            dense_weight: 0.3,
            ..Default::default()
        }
    }

    /// Create a config favoring semantic search
    pub fn semantic_focused() -> Self {
        Self {
            sparse_weight: 0.3,
            dense_weight: 0.7,
            ..Default::default()
        }
    }
}

/// A result from hybrid search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    /// Document ID
    pub id: String,
    /// Combined/fused score
    pub score: f32,
    /// Sparse (BM25) score (if available)
    pub sparse_score: Option<f32>,
    /// Dense (vector) score (if available)
    pub dense_score: Option<f32>,
    /// Sparse rank (1-indexed)
    pub sparse_rank: Option<usize>,
    /// Dense rank (1-indexed)
    pub dense_rank: Option<usize>,
    /// Matching keywords (from BM25)
    pub matching_terms: Vec<String>,
    /// Optional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Vector search result (input from external vector search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Hybrid Search combining sparse and dense retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearch {
    /// Configuration
    pub config: HybridConfig,
    /// BM25 index for sparse search
    bm25_index: Bm25Index,
}

impl HybridSearch {
    /// Create a new hybrid search instance
    pub fn new() -> Self {
        Self::with_config(HybridConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: HybridConfig) -> Self {
        Self {
            config,
            bm25_index: Bm25Index::new(),
        }
    }

    /// Get mutable reference to BM25 index for document management
    pub fn bm25_index_mut(&mut self) -> &mut Bm25Index {
        &mut self.bm25_index
    }

    /// Get reference to BM25 index
    pub fn bm25_index(&self) -> &Bm25Index {
        &self.bm25_index
    }

    /// Perform hybrid search combining BM25 and vector results
    pub fn search(
        &self,
        query: &str,
        vector_results: Vec<VectorResult>,
        top_k: usize,
    ) -> Vec<HybridResult> {
        // Get BM25 results
        let bm25_results = self.bm25_index.search(query, top_k * 2);

        // Combine results based on fusion method
        match self.config.fusion_method {
            FusionMethod::RRF => self.rrf_fusion(&bm25_results, &vector_results, top_k),
            FusionMethod::Weighted => self.weighted_fusion(&bm25_results, &vector_results, top_k),
            FusionMethod::MaxScore => self.max_score_fusion(&bm25_results, &vector_results, top_k),
        }
    }

    /// Reciprocal Rank Fusion
    fn rrf_fusion(
        &self,
        bm25_results: &[Bm25Result],
        vector_results: &[VectorResult],
        top_k: usize,
    ) -> Vec<HybridResult> {
        let k = self.config.rrf_k as f32;
        let mut scores: HashMap<String, HybridResult> = HashMap::new();

        // Process BM25 results
        for (rank, result) in bm25_results.iter().enumerate() {
            let rrf_score = self.config.sparse_weight / (k + (rank + 1) as f32);

            let entry = scores
                .entry(result.id.clone())
                .or_insert_with(|| HybridResult {
                    id: result.id.clone(),
                    score: 0.0,
                    sparse_score: None,
                    dense_score: None,
                    sparse_rank: None,
                    dense_rank: None,
                    matching_terms: Vec::new(),
                    metadata: HashMap::new(),
                });

            entry.score += rrf_score;
            entry.sparse_score = Some(result.score);
            entry.sparse_rank = Some(rank + 1);
            entry.matching_terms = result.matching_terms.clone();
        }

        // Process vector results
        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = self.config.dense_weight / (k + (rank + 1) as f32);

            let entry = scores
                .entry(result.id.clone())
                .or_insert_with(|| HybridResult {
                    id: result.id.clone(),
                    score: 0.0,
                    sparse_score: None,
                    dense_score: None,
                    sparse_rank: None,
                    dense_rank: None,
                    matching_terms: Vec::new(),
                    metadata: result.metadata.clone(),
                });

            entry.score += rrf_score;
            entry.dense_score = Some(result.score);
            entry.dense_rank = Some(rank + 1);
            if entry.metadata.is_empty() {
                entry.metadata = result.metadata.clone();
            }
        }

        self.finalize_results(scores, top_k)
    }

    /// Weighted score combination with normalization
    fn weighted_fusion(
        &self,
        bm25_results: &[Bm25Result],
        vector_results: &[VectorResult],
        top_k: usize,
    ) -> Vec<HybridResult> {
        let mut scores: HashMap<String, HybridResult> = HashMap::new();

        // Normalize BM25 scores (min-max normalization)
        let bm25_max = bm25_results.iter().map(|r| r.score).fold(0.0f32, f32::max);
        let bm25_min = bm25_results
            .iter()
            .map(|r| r.score)
            .fold(f32::MAX, f32::min);
        let bm25_range = (bm25_max - bm25_min).max(0.001);

        // Process BM25 results
        for (rank, result) in bm25_results.iter().enumerate() {
            let normalized_score = (result.score - bm25_min) / bm25_range;
            let weighted_score = normalized_score * self.config.sparse_weight;

            let entry = scores
                .entry(result.id.clone())
                .or_insert_with(|| HybridResult {
                    id: result.id.clone(),
                    score: 0.0,
                    sparse_score: None,
                    dense_score: None,
                    sparse_rank: None,
                    dense_rank: None,
                    matching_terms: Vec::new(),
                    metadata: HashMap::new(),
                });

            entry.score += weighted_score;
            entry.sparse_score = Some(result.score);
            entry.sparse_rank = Some(rank + 1);
            entry.matching_terms = result.matching_terms.clone();
        }

        // Normalize vector scores (already 0-1 for cosine similarity)
        for (rank, result) in vector_results.iter().enumerate() {
            let weighted_score = result.score * self.config.dense_weight;

            let entry = scores
                .entry(result.id.clone())
                .or_insert_with(|| HybridResult {
                    id: result.id.clone(),
                    score: 0.0,
                    sparse_score: None,
                    dense_score: None,
                    sparse_rank: None,
                    dense_rank: None,
                    matching_terms: Vec::new(),
                    metadata: result.metadata.clone(),
                });

            entry.score += weighted_score;
            entry.dense_score = Some(result.score);
            entry.dense_rank = Some(rank + 1);
            if entry.metadata.is_empty() {
                entry.metadata = result.metadata.clone();
            }
        }

        self.finalize_results(scores, top_k)
    }

    /// Max score fusion (take best score from either method)
    fn max_score_fusion(
        &self,
        bm25_results: &[Bm25Result],
        vector_results: &[VectorResult],
        top_k: usize,
    ) -> Vec<HybridResult> {
        let mut scores: HashMap<String, HybridResult> = HashMap::new();

        // Normalize BM25 scores
        let bm25_max = bm25_results.iter().map(|r| r.score).fold(0.0f32, f32::max);
        let bm25_min = bm25_results
            .iter()
            .map(|r| r.score)
            .fold(f32::MAX, f32::min);
        let bm25_range = (bm25_max - bm25_min).max(0.001);

        // Process BM25 results
        for (rank, result) in bm25_results.iter().enumerate() {
            let normalized_score = (result.score - bm25_min) / bm25_range;

            let entry = scores
                .entry(result.id.clone())
                .or_insert_with(|| HybridResult {
                    id: result.id.clone(),
                    score: 0.0,
                    sparse_score: None,
                    dense_score: None,
                    sparse_rank: None,
                    dense_rank: None,
                    matching_terms: Vec::new(),
                    metadata: HashMap::new(),
                });

            entry.score = entry
                .score
                .max(normalized_score * self.config.sparse_weight);
            entry.sparse_score = Some(result.score);
            entry.sparse_rank = Some(rank + 1);
            entry.matching_terms = result.matching_terms.clone();
        }

        // Process vector results
        for (rank, result) in vector_results.iter().enumerate() {
            let entry = scores
                .entry(result.id.clone())
                .or_insert_with(|| HybridResult {
                    id: result.id.clone(),
                    score: 0.0,
                    sparse_score: None,
                    dense_score: None,
                    sparse_rank: None,
                    dense_rank: None,
                    matching_terms: Vec::new(),
                    metadata: result.metadata.clone(),
                });

            entry.score = entry.score.max(result.score * self.config.dense_weight);
            entry.dense_score = Some(result.score);
            entry.dense_rank = Some(rank + 1);
            if entry.metadata.is_empty() {
                entry.metadata = result.metadata.clone();
            }
        }

        self.finalize_results(scores, top_k)
    }

    /// Finalize results: sort, filter, and truncate
    fn finalize_results(
        &self,
        scores: HashMap<String, HybridResult>,
        top_k: usize,
    ) -> Vec<HybridResult> {
        let mut results: Vec<HybridResult> = scores
            .into_values()
            .filter(|r| r.score >= self.config.min_score)
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);

        results
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for HybridSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::bm25::TokenizedDocument;

    fn create_test_hybrid() -> HybridSearch {
        let mut hybrid = HybridSearch::new();
        let config = hybrid.bm25_index().config().clone();

        hybrid.bm25_index_mut().add_documents(vec![
            TokenizedDocument::new(
                "doc1".to_string(),
                "Machine learning is a subset of artificial intelligence".to_string(),
                &config,
            ),
            TokenizedDocument::new(
                "doc2".to_string(),
                "Deep learning neural networks transform technology".to_string(),
                &config,
            ),
            TokenizedDocument::new(
                "doc3".to_string(),
                "Natural language processing enables text understanding".to_string(),
                &config,
            ),
        ]);

        hybrid
    }

    #[test]
    fn test_rrf_fusion() {
        let hybrid = create_test_hybrid();

        // Simulate vector search results
        let vector_results = vec![
            VectorResult {
                id: "doc2".to_string(),
                score: 0.95,
                metadata: HashMap::new(),
            },
            VectorResult {
                id: "doc1".to_string(),
                score: 0.90,
                metadata: HashMap::new(),
            },
            VectorResult {
                id: "doc3".to_string(),
                score: 0.70,
                metadata: HashMap::new(),
            },
        ];

        let results = hybrid.search("machine learning AI", vector_results, 10);

        assert!(!results.is_empty());
        // Results should combine both sparse and dense signals
        for result in &results {
            println!(
                "{}: score={}, sparse={:?}, dense={:?}",
                result.id, result.score, result.sparse_score, result.dense_score
            );
        }
    }

    #[test]
    fn test_weighted_fusion() {
        let mut hybrid = HybridSearch::with_config(HybridConfig {
            fusion_method: FusionMethod::Weighted,
            sparse_weight: 0.4,
            dense_weight: 0.6,
            ..Default::default()
        });

        let config = hybrid.bm25_index().config().clone();
        hybrid.bm25_index_mut().add_document(TokenizedDocument::new(
            "doc1".to_string(),
            "Test document about search".to_string(),
            &config,
        ));

        let vector_results = vec![VectorResult {
            id: "doc1".to_string(),
            score: 0.85,
            metadata: HashMap::new(),
        }];

        let results = hybrid.search("search", vector_results, 10);

        assert_eq!(results.len(), 1);
        // Score should reflect weighted combination
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_documents_only_in_vector() {
        let hybrid = create_test_hybrid();

        // Vector result includes document not in BM25 index
        let vector_results = vec![
            VectorResult {
                id: "doc_new".to_string(),
                score: 0.99,
                metadata: HashMap::new(),
            },
            VectorResult {
                id: "doc1".to_string(),
                score: 0.80,
                metadata: HashMap::new(),
            },
        ];

        let results = hybrid.search("something", vector_results, 10);

        // doc_new should appear only from vector search
        let doc_new = results.iter().find(|r| r.id == "doc_new");
        assert!(doc_new.is_some());
        assert!(doc_new.unwrap().sparse_score.is_none());
        assert!(doc_new.unwrap().dense_score.is_some());
    }
}
