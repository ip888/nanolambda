//! Rate limiting for API keys using token bucket algorithm

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Maximum tokens (capacity)
    capacity: u32,
    /// Current tokens
    tokens: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume tokens, returns true if allowed
    pub fn try_consume(&mut self, tokens: u32) -> bool {
        self.refill();
        
        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Get current token count
    pub fn available_tokens(&mut self) -> u32 {
        self.refill();
        self.tokens as u32
    }

    /// Get time until next token available (if bucket is empty)
    pub fn time_until_available(&mut self) -> Duration {
        self.refill();
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }
}

/// Rate limiting tier configuration
#[derive(Debug, Clone, Copy)]
pub enum RateLimitTier {
    /// Free tier: 10 req/min (600/hour)
    Free,
    /// Hobby tier: 100 req/min (6000/hour)
    Hobby,
    /// Developer tier: 500 req/min (30000/hour)
    Developer,
    /// Production tier: 2000 req/min (120000/hour)
    Production,
    /// Enterprise tier: unlimited (high capacity)
    Enterprise,
}

impl RateLimitTier {
    /// Get bucket configuration (capacity, refill_rate per second)
    pub fn bucket_config(&self) -> (u32, f64) {
        match self {
            RateLimitTier::Free => (20, 10.0 / 60.0),        // 10/min burst 20
            RateLimitTier::Hobby => (200, 100.0 / 60.0),     // 100/min burst 200
            RateLimitTier::Developer => (1000, 500.0 / 60.0), // 500/min burst 1000
            RateLimitTier::Production => (4000, 2000.0 / 60.0), // 2000/min burst 4000
            RateLimitTier::Enterprise => (10000, 5000.0 / 60.0), // 5000/min burst 10000
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RateLimitTier::Free => "free",
            RateLimitTier::Hobby => "hobby",
            RateLimitTier::Developer => "developer",
            RateLimitTier::Production => "production",
            RateLimitTier::Enterprise => "enterprise",
        }
    }
}

/// Rate limiter managing buckets for multiple API keys
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, (TokenBucket, RateLimitTier)>>>,
    default_tier: RateLimitTier,
}

impl RateLimiter {
    pub fn new(default_tier: RateLimitTier) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            default_tier,
        }
    }

    /// Check if request is allowed for given API key
    pub async fn check_rate_limit(&self, api_key: &str) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.write().await;
        
        let (bucket, tier) = buckets
            .entry(api_key.to_string())
            .or_insert_with(|| {
                let (capacity, rate) = self.default_tier.bucket_config();
                (TokenBucket::new(capacity, rate), self.default_tier)
            });

        if bucket.try_consume(1) {
            Ok(())
        } else {
            let retry_after = bucket.time_until_available();
            Err(RateLimitError::RateLimitExceeded {
                tier: *tier,
                retry_after,
            })
        }
    }

    /// Set tier for specific API key
    pub async fn set_tier(&self, api_key: &str, tier: RateLimitTier) {
        let mut buckets = self.buckets.write().await;
        let (capacity, rate) = tier.bucket_config();
        buckets.insert(api_key.to_string(), (TokenBucket::new(capacity, rate), tier));
    }

    /// Get current rate limit status for API key
    pub async fn get_status(&self, api_key: &str) -> RateLimitStatus {
        let mut buckets = self.buckets.write().await;
        
        if let Some((bucket, tier)) = buckets.get_mut(api_key) {
            RateLimitStatus {
                tier: *tier,
                available_tokens: bucket.available_tokens(),
                capacity: bucket.capacity,
                refill_rate: bucket.refill_rate,
            }
        } else {
            let (capacity, rate) = self.default_tier.bucket_config();
            RateLimitStatus {
                tier: self.default_tier,
                available_tokens: capacity,
                capacity,
                refill_rate: rate,
            }
        }
    }

    /// Get statistics for all API keys
    pub async fn get_all_stats(&self) -> Vec<(String, RateLimitStatus)> {
        let mut buckets = self.buckets.write().await;
        buckets
            .iter_mut()
            .map(|(key, (bucket, tier))| {
                (
                    key.clone(),
                    RateLimitStatus {
                        tier: *tier,
                        available_tokens: bucket.available_tokens(),
                        capacity: bucket.capacity,
                        refill_rate: bucket.refill_rate,
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum RateLimitError {
    RateLimitExceeded {
        tier: RateLimitTier,
        retry_after: Duration,
    },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::RateLimitExceeded { tier, retry_after } => {
                write!(
                    f,
                    "Rate limit exceeded for {} tier. Retry after {:.1}s",
                    tier.name(),
                    retry_after.as_secs_f64()
                )
            }
        }
    }
}

impl std::error::Error for RateLimitError {}

#[derive(Debug, serde::Serialize)]
pub struct RateLimitStatus {
    pub tier: RateLimitTier,
    pub available_tokens: u32,
    pub capacity: u32,
    pub refill_rate: f64,
}

// Make RateLimitTier serializable
impl serde::Serialize for RateLimitTier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 1.0); // 10 capacity, 1/sec
        
        // Can consume up to capacity
        assert!(bucket.try_consume(5));
        assert_eq!(bucket.available_tokens(), 5);
        
        // Can't exceed capacity
        assert!(!bucket.try_consume(10));
        assert!(bucket.try_consume(5));
        assert_eq!(bucket.available_tokens(), 0);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(RateLimitTier::Free);
        
        // Should allow first requests
        assert!(limiter.check_rate_limit("test_key").await.is_ok());
        
        // Can check status
        let status = limiter.get_status("test_key").await;
        assert!(status.available_tokens < status.capacity);
    }
}
