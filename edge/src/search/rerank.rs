//! Reranking Module
//!
//! Provides intelligent result reranking using multiple scoring signals.
//! Similar to Cohere's Rerank API but built-in and customizable.
//!
//! Features:
//! - Cross-encoder style scoring (query-document pairs)
//! - Multiple scoring signals: semantic, keyword overlap, recency, popularity
//! - Configurable signal weights
//! - Metadata-based boosting

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// Reranker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerConfig {
    /// Weight for semantic similarity signal
    pub semantic_weight: f32,
    /// Weight for keyword overlap signal
    pub keyword_weight: f32,
    /// Weight for recency signal (newer = higher)
    pub recency_weight: f32,
    /// Weight for popularity signal
    pub popularity_weight: f32,
    /// Minimum score threshold
    pub min_score: f32,
    /// Diversity penalty for similar consecutive results
    pub diversity_penalty: f32,
    /// Maximum age in hours for recency calculation (older = 0)
    pub max_age_hours: u64,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.5,
            keyword_weight: 0.3,
            recency_weight: 0.1,
            popularity_weight: 0.1,
            min_score: 0.0,
            diversity_penalty: 0.0,
            max_age_hours: 720, // 30 days
        }
    }
}

impl RerankerConfig {
    /// Config optimized for semantic relevance
    pub fn semantic_focused() -> Self {
        Self {
            semantic_weight: 0.7,
            keyword_weight: 0.2,
            recency_weight: 0.05,
            popularity_weight: 0.05,
            ..Default::default()
        }
    }
    
    /// Config optimized for exact keyword matching
    pub fn keyword_focused() -> Self {
        Self {
            semantic_weight: 0.3,
            keyword_weight: 0.5,
            recency_weight: 0.1,
            popularity_weight: 0.1,
            ..Default::default()
        }
    }
    
    /// Config optimized for freshness
    pub fn recency_focused() -> Self {
        Self {
            semantic_weight: 0.4,
            keyword_weight: 0.2,
            recency_weight: 0.3,
            popularity_weight: 0.1,
            ..Default::default()
        }
    }
}

/// Document to be reranked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerDocument {
    /// Document ID
    pub id: String,
    /// Document text content
    pub text: String,
    /// Pre-computed vector embedding (optional)
    pub embedding: Option<Vec<f32>>,
    /// Original/initial score
    pub initial_score: f32,
    /// Document creation/update time
    pub timestamp: Option<DateTime<Utc>>,
    /// Popularity metric (views, clicks, etc.)
    pub popularity: Option<u64>,
    /// Metadata for custom boosting
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RerankerDocument {
    /// Create a minimal document for reranking
    pub fn new(id: String, text: String, initial_score: f32) -> Self {
        Self {
            id,
            text,
            embedding: None,
            initial_score,
            timestamp: None,
            popularity: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Add embedding to document
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }
    
    /// Add timestamp to document
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    
    /// Add popularity score
    pub fn with_popularity(mut self, popularity: u64) -> Self {
        self.popularity = Some(popularity);
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }
}

/// A ranked result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    /// Document ID
    pub id: String,
    /// Final reranked score (0.0-1.0)
    pub relevance_score: f32,
    /// Original position before reranking (0-indexed)
    pub original_index: usize,
    /// Component scores breakdown
    pub score_breakdown: ScoreBreakdown,
    /// Document text (if requested)
    pub text: Option<String>,
}

/// Breakdown of component scores
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBreakdown {
    /// Semantic similarity component
    pub semantic: f32,
    /// Keyword overlap component  
    pub keyword: f32,
    /// Recency component
    pub recency: f32,
    /// Popularity component
    pub popularity: f32,
}

/// Reranker for intelligent result ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reranker {
    /// Configuration
    pub config: RerankerConfig,
}

impl Default for Reranker {
    fn default() -> Self {
        Self::new()
    }
}

impl Reranker {
    /// Create a new reranker with default config
    pub fn new() -> Self {
        Self {
            config: RerankerConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: RerankerConfig) -> Self {
        Self { config }
    }
    
    /// Rerank documents given a query
    pub fn rerank(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        documents: Vec<RerankerDocument>,
        top_k: usize,
        include_text: bool,
    ) -> Vec<RankedResult> {
        let query_terms: HashSet<String> = self.tokenize(query);
        
        // Calculate max popularity for normalization
        let max_popularity = documents.iter()
            .filter_map(|d| d.popularity)
            .max()
            .unwrap_or(1) as f32;
        
        let mut results: Vec<RankedResult> = documents
            .into_iter()
            .enumerate()
            .map(|(idx, doc)| {
                let breakdown = self.calculate_scores(
                    &query_terms,
                    query_embedding,
                    &doc,
                    max_popularity,
                );
                
                let relevance_score = self.combine_scores(&breakdown);
                
                RankedResult {
                    id: doc.id.clone(),
                    relevance_score,
                    original_index: idx,
                    score_breakdown: breakdown,
                    text: if include_text { Some(doc.text) } else { None },
                }
            })
            .filter(|r| r.relevance_score >= self.config.min_score)
            .collect();
        
        // Sort by relevance score descending
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Apply diversity penalty if configured
        if self.config.diversity_penalty > 0.0 {
            self.apply_diversity_penalty(&mut results);
        }
        
        results.truncate(top_k);
        results
    }
    
    /// Calculate component scores for a document
    fn calculate_scores(
        &self,
        query_terms: &HashSet<String>,
        query_embedding: Option<&[f32]>,
        doc: &RerankerDocument,
        max_popularity: f32,
    ) -> ScoreBreakdown {
        let mut breakdown = ScoreBreakdown::default();
        
        // Semantic similarity (if embeddings available)
        if let (Some(query_emb), Some(doc_emb)) = (query_embedding, &doc.embedding) {
            breakdown.semantic = cosine_similarity(query_emb, doc_emb);
        } else {
            // Use initial score as proxy for semantic similarity
            breakdown.semantic = doc.initial_score;
        }
        
        // Keyword overlap (Jaccard-like)
        let doc_terms: HashSet<String> = self.tokenize(&doc.text);
        breakdown.keyword = jaccard_similarity(query_terms, &doc_terms);
        
        // Recency score
        if let Some(timestamp) = doc.timestamp {
            breakdown.recency = self.calculate_recency(timestamp);
        } else {
            breakdown.recency = 0.5; // Neutral if no timestamp
        }
        
        // Popularity score (normalized)
        if let Some(popularity) = doc.popularity {
            breakdown.popularity = (popularity as f32) / max_popularity;
        } else {
            breakdown.popularity = 0.0;
        }
        
        breakdown
    }
    
    /// Combine component scores using configured weights
    fn combine_scores(&self, breakdown: &ScoreBreakdown) -> f32 {
        let c = &self.config;
        
        c.semantic_weight * breakdown.semantic +
        c.keyword_weight * breakdown.keyword +
        c.recency_weight * breakdown.recency +
        c.popularity_weight * breakdown.popularity
    }
    
    /// Calculate recency score (1.0 for now, 0.0 for max_age_hours ago)
    fn calculate_recency(&self, timestamp: DateTime<Utc>) -> f32 {
        let now = Utc::now();
        let age_hours = (now - timestamp).num_hours().max(0) as f32;
        let max_age = self.config.max_age_hours as f32;
        
        if age_hours >= max_age {
            0.0
        } else {
            1.0 - (age_hours / max_age)
        }
    }
    
    /// Apply diversity penalty to reduce similar consecutive results
    fn apply_diversity_penalty(&self, results: &mut [RankedResult]) {
        // Simple approach: penalize results with similar scores to previous
        for i in 1..results.len() {
            let prev_score = results[i - 1].relevance_score;
            let curr_score = results[i].relevance_score;
            
            if (prev_score - curr_score).abs() < 0.05 {
                results[i].relevance_score *= 1.0 - self.config.diversity_penalty;
            }
        }
        
        // Re-sort after penalty
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Tokenize text for keyword matching
    fn tokenize(&self, text: &str) -> HashSet<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 2)
            .map(|s| s.to_string())
            .collect()
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

/// Calculate Jaccard similarity between two sets
fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    
    if union == 0 {
        return 0.0;
    }
    
    intersection as f32 / union as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    
    #[test]
    fn test_basic_reranking() {
        let reranker = Reranker::new();
        
        let docs = vec![
            RerankerDocument::new("doc1".to_string(), "Machine learning AI".to_string(), 0.8),
            RerankerDocument::new("doc2".to_string(), "Weather forecast today".to_string(), 0.9),
            RerankerDocument::new("doc3".to_string(), "Deep learning neural networks".to_string(), 0.7),
        ];
        
        let results = reranker.rerank("machine learning", None, docs, 10, false);
        
        assert!(!results.is_empty());
        // doc1 and doc3 should rank higher due to keyword match
        let top_ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(top_ids[0] == "doc1" || top_ids[0] == "doc3");
    }
    
    #[test]
    fn test_recency_scoring() {
        let mut config = RerankerConfig::default();
        config.recency_weight = 0.9;  // Heavily weight recency
        config.semantic_weight = 0.1;
        config.keyword_weight = 0.0;
        config.popularity_weight = 0.0;
        
        let reranker = Reranker::with_config(config);
        
        let now = Utc::now();
        let docs = vec![
            RerankerDocument::new("old".to_string(), "test".to_string(), 0.9)
                .with_timestamp(now - Duration::hours(500)),
            RerankerDocument::new("new".to_string(), "test".to_string(), 0.5)
                .with_timestamp(now - Duration::hours(1)),
        ];
        
        let results = reranker.rerank("test", None, docs, 10, false);
        
        // New doc should rank first due to recency
        assert_eq!(results[0].id, "new");
    }
    
    #[test]
    fn test_keyword_overlap() {
        let config = RerankerConfig::keyword_focused();
        let reranker = Reranker::with_config(config);
        
        let docs = vec![
            RerankerDocument::new("doc1".to_string(), "apple banana cherry".to_string(), 0.9),
            RerankerDocument::new("doc2".to_string(), "apple banana date".to_string(), 0.7),
            RerankerDocument::new("doc3".to_string(), "grape melon orange".to_string(), 0.8),
        ];
        
        let results = reranker.rerank("apple banana", None, docs, 10, true);
        
        // doc1 and doc2 should rank higher than doc3
        assert!(results[0].id == "doc1" || results[0].id == "doc2");
        assert!(results[1].id == "doc1" || results[1].id == "doc2");
    }
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }
    
    #[test]
    fn test_jaccard_similarity() {
        let a: HashSet<String> = ["apple", "banana"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<String> = ["apple", "banana", "cherry"].iter().map(|s| s.to_string()).collect();
        
        // Intersection: 2, Union: 3 -> 2/3 = 0.666...
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 0.666).abs() < 0.01);
    }
}
