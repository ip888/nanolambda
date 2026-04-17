//! Smart Edge Caching
//!
//! Advanced caching with:
//! - Stale-while-revalidate
//! - Cache warming
//! - Adaptive TTL
//! - Request coalescing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::Url;

/// Cache entry with advanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCacheEntry {
    /// Cached value
    pub value: Vec<u8>,
    /// Content type
    pub content_type: String,
    /// Created timestamp
    pub created_at: u64,
    /// Expires timestamp
    pub expires_at: u64,
    /// Stale-while-revalidate window (ms after expiry)
    pub stale_window: u64,
    /// Number of hits
    pub hits: u64,
    /// Last accessed timestamp
    pub last_accessed: u64,
    /// ETag for validation
    pub etag: Option<String>,
    /// Custom headers to cache
    pub headers: HashMap<String, String>,
    /// Cache tags for invalidation
    pub tags: Vec<String>,
}

impl SmartCacheEntry {
    /// Check if entry is fresh
    pub fn is_fresh(&self, now: u64) -> bool {
        now < self.expires_at
    }

    /// Check if entry is stale but usable
    pub fn is_stale_usable(&self, now: u64) -> bool {
        now >= self.expires_at && now < self.expires_at + self.stale_window
    }

    /// Check if entry is expired
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at + self.stale_window
    }

    /// Get age in milliseconds
    pub fn age_ms(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }
}

/// Cache policy configuration
#[derive(Debug, Clone)]
pub struct CachePolicy {
    /// Default TTL in milliseconds
    pub default_ttl_ms: u64,
    /// Stale-while-revalidate window
    pub stale_window_ms: u64,
    /// Maximum entry size in bytes
    pub max_entry_size: usize,
    /// Maximum total cache size
    pub max_total_size: usize,
    /// Enable adaptive TTL
    pub adaptive_ttl: bool,
    /// Minimum TTL for adaptive
    pub min_ttl_ms: u64,
    /// Maximum TTL for adaptive
    pub max_ttl_ms: u64,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            default_ttl_ms: 60_000, // 1 minute
            stale_window_ms: 30_000, // 30 seconds
            max_entry_size: 5 * 1024 * 1024, // 5MB
            max_total_size: 128 * 1024 * 1024, // 128MB
            adaptive_ttl: true,
            min_ttl_ms: 10_000, // 10 seconds
            max_ttl_ms: 3600_000, // 1 hour
        }
    }
}

/// Smart cache result
#[derive(Debug)]
pub enum CacheResult {
    /// Fresh hit
    Hit(SmartCacheEntry),
    /// Stale hit (should revalidate in background)
    Stale(SmartCacheEntry),
    /// Cache miss
    Miss,
}

/// Request coalescer to prevent thundering herd
pub struct RequestCoalescer {
    /// Pending requests by key
    pending: HashMap<String, PendingRequest>,
}

/// Pending request info
struct PendingRequest {
    /// Request started at
    started_at: u64,
    /// Number of waiting requests
    waiters: u32,
}

impl RequestCoalescer {
    /// Create new coalescer
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    /// Check if request is already pending
    pub fn is_pending(&self, key: &str) -> bool {
        self.pending.contains_key(key)
    }

    /// Start a new request
    pub fn start(&mut self, key: String, now: u64) -> bool {
        if self.pending.contains_key(&key) {
            // Increment waiter count
            if let Some(pending) = self.pending.get_mut(&key) {
                pending.waiters += 1;
            }
            false
        } else {
            self.pending.insert(
                key,
                PendingRequest {
                    started_at: now,
                    waiters: 1,
                },
            );
            true
        }
    }

    /// Complete a request
    pub fn complete(&mut self, key: &str) -> u32 {
        self.pending
            .remove(key)
            .map(|p| p.waiters)
            .unwrap_or(0)
    }

    /// Get waiter count
    pub fn waiter_count(&self, key: &str) -> u32 {
        self.pending.get(key).map(|p| p.waiters).unwrap_or(0)
    }
}

impl Default for RequestCoalescer {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive TTL calculator
pub struct AdaptiveTtl {
    /// Policy
    policy: CachePolicy,
    /// Hit rate per key (recent)
    hit_rates: HashMap<String, HitRateTracker>,
}

/// Hit rate tracker
struct HitRateTracker {
    hits: u32,
    misses: u32,
    last_reset: u64,
}

impl HitRateTracker {
    fn new(now: u64) -> Self {
        Self {
            hits: 0,
            misses: 0,
            last_reset: now,
        }
    }

    fn record_hit(&mut self) {
        self.hits += 1;
    }

    fn record_miss(&mut self) {
        self.misses += 1;
    }

    fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.5 // Default
        } else {
            self.hits as f64 / total as f64
        }
    }

    fn should_reset(&self, now: u64) -> bool {
        // Reset every 5 minutes
        now - self.last_reset > 300_000
    }
}

impl AdaptiveTtl {
    /// Create new adaptive TTL calculator
    pub fn new(policy: CachePolicy) -> Self {
        Self {
            policy,
            hit_rates: HashMap::new(),
        }
    }

    /// Record a cache hit
    pub fn record_hit(&mut self, key: &str, now: u64) {
        let tracker = self
            .hit_rates
            .entry(key.to_string())
            .or_insert_with(|| HitRateTracker::new(now));

        if tracker.should_reset(now) {
            *tracker = HitRateTracker::new(now);
        }

        tracker.record_hit();
    }

    /// Record a cache miss
    pub fn record_miss(&mut self, key: &str, now: u64) {
        let tracker = self
            .hit_rates
            .entry(key.to_string())
            .or_insert_with(|| HitRateTracker::new(now));

        if tracker.should_reset(now) {
            *tracker = HitRateTracker::new(now);
        }

        tracker.record_miss();
    }

    /// Calculate adaptive TTL for a key
    pub fn calculate_ttl(&self, key: &str) -> u64 {
        if !self.policy.adaptive_ttl {
            return self.policy.default_ttl_ms;
        }

        let hit_rate = self
            .hit_rates
            .get(key)
            .map(|t| t.hit_rate())
            .unwrap_or(0.5);

        // Higher hit rate = longer TTL
        let range = self.policy.max_ttl_ms - self.policy.min_ttl_ms;
        let ttl = self.policy.min_ttl_ms + (range as f64 * hit_rate) as u64;

        ttl.clamp(self.policy.min_ttl_ms, self.policy.max_ttl_ms)
    }
}

/// Cache warming configuration
#[derive(Debug, Clone)]
pub struct WarmingConfig {
    /// Keys to warm on startup
    pub warm_keys: Vec<String>,
    /// Refresh interval for warm keys
    pub refresh_interval_ms: u64,
    /// Parallel warming requests
    pub parallel_requests: usize,
}

impl Default for WarmingConfig {
    fn default() -> Self {
        Self {
            warm_keys: Vec::new(),
            refresh_interval_ms: 60_000,
            parallel_requests: 5,
        }
    }
}

/// Cache warmer
pub struct CacheWarmer {
    config: WarmingConfig,
    /// Last warm time per key
    last_warmed: HashMap<String, u64>,
}

impl CacheWarmer {
    /// Create new cache warmer
    pub fn new(config: WarmingConfig) -> Self {
        Self {
            config,
            last_warmed: HashMap::new(),
        }
    }

    /// Get keys that need warming
    pub fn keys_to_warm(&self, now: u64) -> Vec<String> {
        self.config
            .warm_keys
            .iter()
            .filter(|key| {
                self.last_warmed
                    .get(*key)
                    .map(|last| now - last > self.config.refresh_interval_ms)
                    .unwrap_or(true)
            })
            .take(self.config.parallel_requests)
            .cloned()
            .collect()
    }

    /// Mark key as warmed
    pub fn mark_warmed(&mut self, key: &str, now: u64) {
        self.last_warmed.insert(key.to_string(), now);
    }
}

/// Cache key generator with normalization
pub struct CacheKeyGenerator {
    /// Include query params in key
    pub include_query: bool,
    /// Query params to exclude
    pub exclude_params: Vec<String>,
    /// Include headers in key
    pub include_headers: Vec<String>,
}

impl Default for CacheKeyGenerator {
    fn default() -> Self {
        Self {
            include_query: true,
            exclude_params: vec![
                "utm_source".to_string(),
                "utm_medium".to_string(),
                "utm_campaign".to_string(),
                "fbclid".to_string(),
                "gclid".to_string(),
                "_".to_string(), // Cache busting param
            ],
            include_headers: Vec::new(),
        }
    }
}

impl CacheKeyGenerator {
    /// Generate cache key from URL
    pub fn generate(&self, url: &str, headers: Option<&HashMap<String, String>>) -> String {
        let mut hasher = SimpleHasher::new();

        // Parse URL
        if let Ok(parsed) = Url::parse(url) {
            // Add path
            hasher.update(parsed.path().as_bytes());

            // Add filtered query params
            if self.include_query {
                let mut params: Vec<_> = parsed
                    .query_pairs()
                    .filter(|(k, _)| !self.exclude_params.contains(&k.to_string()))
                    .collect();
                params.sort_by(|a, b| a.0.cmp(&b.0));

                for (k, v) in params {
                    hasher.update(k.as_bytes());
                    hasher.update(v.as_bytes());
                }
            }
        } else {
            hasher.update(url.as_bytes());
        }

        // Add headers
        if let Some(hdrs) = headers {
            for key in &self.include_headers {
                if let Some(val) = hdrs.get(key) {
                    hasher.update(key.as_bytes());
                    hasher.update(val.as_bytes());
                }
            }
        }

        hasher.finish_hex()
    }
}

/// Simple hasher (FNV-1a)
struct SimpleHasher {
    hash: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        Self {
            hash: 0xcbf29ce484222325, // FNV offset basis
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= *byte as u64;
            self.hash = self.hash.wrapping_mul(0x100000001b3); // FNV prime
        }
    }

    fn finish_hex(&self) -> String {
        format!("{:016x}", self.hash)
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Stale hits
    pub stale_hits: u64,
    /// Evictions
    pub evictions: u64,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Entry count
    pub entry_count: u64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_freshness() {
        let entry = SmartCacheEntry {
            value: vec![1, 2, 3],
            content_type: "application/json".to_string(),
            created_at: 1000,
            expires_at: 2000,
            stale_window: 500,
            hits: 0,
            last_accessed: 1000,
            etag: None,
            headers: HashMap::new(),
            tags: Vec::new(),
        };

        assert!(entry.is_fresh(1500));
        assert!(!entry.is_fresh(2500));
        assert!(entry.is_stale_usable(2200));
        assert!(entry.is_expired(2600));
    }

    #[test]
    fn test_request_coalescer() {
        let mut coalescer = RequestCoalescer::new();

        // First request should start
        assert!(coalescer.start("key1".to_string(), 1000));

        // Second request should wait
        assert!(!coalescer.start("key1".to_string(), 1001));

        // Should have 2 waiters
        assert_eq!(coalescer.waiter_count("key1"), 2);

        // Complete returns waiter count
        assert_eq!(coalescer.complete("key1"), 2);
    }

    #[test]
    fn test_adaptive_ttl() {
        let policy = CachePolicy {
            adaptive_ttl: true,
            min_ttl_ms: 10_000,
            max_ttl_ms: 60_000,
            ..Default::default()
        };

        let mut adaptive = AdaptiveTtl::new(policy);

        // Record many hits
        for i in 0..10 {
            adaptive.record_hit("popular", i * 100);
        }

        // Record many misses
        for i in 0..10 {
            adaptive.record_miss("unpopular", i * 100);
        }

        let popular_ttl = adaptive.calculate_ttl("popular");
        let unpopular_ttl = adaptive.calculate_ttl("unpopular");

        assert!(popular_ttl > unpopular_ttl);
    }

    #[test]
    fn test_cache_key_generator() {
        let generator = CacheKeyGenerator::default();

        let key1 = generator.generate("https://api.example.com/users?id=1", None);
        let key2 = generator.generate("https://api.example.com/users?id=1&utm_source=test", None);

        // Should be same (utm params excluded)
        assert_eq!(key1, key2);

        let key3 = generator.generate("https://api.example.com/users?id=2", None);
        assert_ne!(key1, key3);
    }
}
