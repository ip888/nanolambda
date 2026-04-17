//! Semantic Cache Implementation
//!
//! A semantic caching layer that stores AI responses indexed by embedding vectors.
//! When a new query comes in, it's embedded and compared against cached entries.
//! If a similar query is found (above similarity threshold), the cached response
//! is returned instead of calling the AI model.
//!
//! Key Features:
//! - Configurable similarity threshold (default: 0.95)
//! - TTL-based expiration
//! - LRU eviction when cache is full
//! - Namespace isolation per user
//! - Hit rate tracking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Semantic cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Minimum similarity score (0.0-1.0) for cache hit
    pub similarity_threshold: f32,
    /// Maximum number of entries per namespace
    pub max_entries: usize,
    /// Time-to-live in seconds (0 = no expiration)
    pub ttl_seconds: u64,
    /// Whether to include embedding in response
    pub include_embedding: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.95,
            max_entries: 1000,
            ttl_seconds: 3600, // 1 hour default
            include_embedding: false,
        }
    }
}

/// A cached entry containing query embedding and response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Unique entry ID
    pub id: String,
    /// Original query text
    pub query: String,
    /// Query embedding vector
    pub embedding: Vec<f32>,
    /// Cached response (AI completion, chat response, etc.)
    pub response: serde_json::Value,
    /// Model used to generate the response
    pub model: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Number of times this entry was hit
    pub hit_count: u64,
    /// Optional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(
        id: String,
        query: String,
        embedding: Vec<f32>,
        response: serde_json::Value,
        model: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            query,
            embedding,
            response,
            model,
            created_at: now,
            last_accessed: now,
            hit_count: 0,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if entry has expired
    pub fn is_expired(&self, ttl_seconds: u64) -> bool {
        if ttl_seconds == 0 {
            return false;
        }
        let expiry = self.created_at + Duration::seconds(ttl_seconds as i64);
        Utc::now() > expiry
    }
    
    /// Update access timestamp and increment hit count
    pub fn record_hit(&mut self) {
        self.last_accessed = Utc::now();
        self.hit_count += 1;
    }
}

/// Cache hit result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHit {
    /// The matched cache entry
    pub entry: CacheEntry,
    /// Similarity score (0.0-1.0)
    pub similarity: f32,
    /// Time saved by cache hit (estimated latency reduction)
    pub latency_saved_ms: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// Total cache queries
    pub total_queries: u64,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of entries
    pub entry_count: usize,
    /// Total bytes used (approximate)
    pub bytes_used: usize,
    /// Average similarity score on hits
    pub avg_hit_similarity: f32,
}

impl CacheStats {
    /// Calculate hit rate as percentage
    pub fn hit_rate(&self) -> f32 {
        if self.total_queries == 0 {
            0.0
        } else {
            (self.hits as f32 / self.total_queries as f32) * 100.0
        }
    }
}

/// Semantic Cache for AI responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCache {
    /// Configuration
    pub config: CacheConfig,
    /// Cache entries indexed by ID
    entries: HashMap<String, CacheEntry>,
    /// Statistics
    stats: CacheStats,
    /// Namespace (for isolation)
    namespace: String,
}

impl SemanticCache {
    /// Create a new semantic cache with default configuration
    pub fn new(namespace: String) -> Self {
        Self::with_config(namespace, CacheConfig::default())
    }
    
    /// Create a new semantic cache with custom configuration
    pub fn with_config(namespace: String, config: CacheConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            stats: CacheStats::default(),
            namespace,
        }
    }
    
    /// Query the cache for a semantically similar entry
    pub fn query(&mut self, query_embedding: &[f32]) -> Option<CacheHit> {
        self.stats.total_queries += 1;
        
        let mut best_match: Option<(String, f32)> = None;
        
        // Find the most similar entry above threshold
        for (id, entry) in &self.entries {
            // Skip expired entries
            if entry.is_expired(self.config.ttl_seconds) {
                continue;
            }
            
            let similarity = cosine_similarity(query_embedding, &entry.embedding);
            
            if similarity >= self.config.similarity_threshold {
                match &best_match {
                    None => best_match = Some((id.clone(), similarity)),
                    Some((_, best_sim)) if similarity > *best_sim => {
                        best_match = Some((id.clone(), similarity));
                    }
                    _ => {}
                }
            }
        }
        
        if let Some((id, similarity)) = best_match {
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.record_hit();
                self.stats.hits += 1;
                
                // Update average similarity
                let total_hits = self.stats.hits as f32;
                self.stats.avg_hit_similarity = 
                    (self.stats.avg_hit_similarity * (total_hits - 1.0) + similarity) / total_hits;
                
                return Some(CacheHit {
                    entry: entry.clone(),
                    similarity,
                    latency_saved_ms: 500, // Estimated AI call latency
                });
            }
        }
        
        self.stats.misses += 1;
        None
    }
    
    /// Add an entry to the cache
    pub fn insert(&mut self, entry: CacheEntry) {
        // Evict if at capacity
        if self.entries.len() >= self.config.max_entries {
            self.evict_lru();
        }
        
        // Clean up expired entries
        self.cleanup_expired();
        
        // Update stats
        let entry_size = std::mem::size_of::<CacheEntry>() 
            + entry.embedding.len() * std::mem::size_of::<f32>()
            + entry.query.len()
            + entry.response.to_string().len();
        self.stats.bytes_used += entry_size;
        
        self.entries.insert(entry.id.clone(), entry);
        self.stats.entry_count = self.entries.len();
    }
    
    /// Remove a specific entry
    pub fn remove(&mut self, id: &str) -> Option<CacheEntry> {
        let entry = self.entries.remove(id);
        self.stats.entry_count = self.entries.len();
        entry
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats.entry_count = 0;
        self.stats.bytes_used = 0;
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
    
    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Evict the least recently used entry
    fn evict_lru(&mut self) {
        let lru_id = self.entries.iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(id, _)| id.clone());
        
        if let Some(id) = lru_id {
            self.entries.remove(&id);
        }
    }
    
    /// Remove all expired entries
    fn cleanup_expired(&mut self) {
        let ttl = self.config.ttl_seconds;
        self.entries.retain(|_, entry| !entry.is_expired(ttl));
        self.stats.entry_count = self.entries.len();
    }
    
    /// Serialize the cache to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    /// Deserialize the cache from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    
    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    
    for (x, y) in a.iter().zip(b.iter()) {
        dot_product += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_hit() {
        let mut cache = SemanticCache::new("test".to_string());
        cache.config.similarity_threshold = 0.9;
        
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let entry = CacheEntry::new(
            "entry1".to_string(),
            "What is the weather?".to_string(),
            embedding.clone(),
            serde_json::json!({"response": "It's sunny"}),
            "test-model".to_string(),
        );
        
        cache.insert(entry);
        
        // Query with exact same embedding should hit
        let result = cache.query(&embedding);
        assert!(result.is_some());
        let hit = result.unwrap();
        assert!((hit.similarity - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_cache_miss() {
        let mut cache = SemanticCache::new("test".to_string());
        cache.config.similarity_threshold = 0.9;
        
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let entry = CacheEntry::new(
            "entry1".to_string(),
            "What is the weather?".to_string(),
            embedding,
            serde_json::json!({"response": "It's sunny"}),
            "test-model".to_string(),
        );
        
        cache.insert(entry);
        
        // Query with very different embedding should miss
        let different_embedding = vec![0.9, 0.8, 0.7, 0.6, 0.5];
        let result = cache.query(&different_embedding);
        assert!(result.is_none());
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
    fn test_lru_eviction() {
        let mut cache = SemanticCache::new("test".to_string());
        cache.config.max_entries = 2;
        
        // Insert 3 entries, oldest should be evicted
        for i in 0..3 {
            let entry = CacheEntry::new(
                format!("entry{}", i),
                format!("Query {}", i),
                vec![i as f32; 5],
                serde_json::json!({"i": i}),
                "test".to_string(),
            );
            cache.insert(entry);
        }
        
        assert_eq!(cache.len(), 2);
    }
}
