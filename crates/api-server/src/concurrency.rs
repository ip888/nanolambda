//! Concurrency control for function invocations
//!
//! This module provides global and per-function concurrency limits to:
//! - Prevent resource exhaustion
//! - Enable reliable SLAs
//! - Support fair scheduling across functions
//! - Queue overflow requests for later processing

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

/// Configuration for concurrency limits
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// Maximum concurrent executions globally
    pub max_global_concurrent: usize,

    /// Maximum concurrent executions per function
    pub max_per_function: usize,

    /// Maximum size of request queue
    pub max_queue_size: usize,

    /// Timeout for waiting in queue
    pub queue_timeout: Duration,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_global_concurrent: 100,
            max_per_function: 10,
            max_queue_size: 1000,
            queue_timeout: Duration::from_secs(30),
        }
    }
}

/// Statistics for a function's concurrency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionConcurrencyStats {
    /// Current number of running invocations
    pub current_running: usize,

    /// Number of requests currently queued
    pub current_queued: usize,

    /// Total requests processed
    pub total_processed: u64,

    /// Total requests rejected (queue full)
    pub total_rejected: u64,

    /// Total requests timed out in queue
    pub total_timeouts: u64,

    /// Average wait time in queue (ms)
    pub avg_queue_wait_ms: f64,
}

/// Per-function concurrency tracking
struct FunctionConcurrency {
    /// Semaphore for per-function limit
    semaphore: Arc<Semaphore>,

    /// Statistics
    stats: FunctionConcurrencyStats,

    /// Last activity timestamp
    last_used: Instant,
}

impl FunctionConcurrency {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            stats: FunctionConcurrencyStats {
                current_running: 0,
                current_queued: 0,
                total_processed: 0,
                total_rejected: 0,
                total_timeouts: 0,
                avg_queue_wait_ms: 0.0,
            },
            last_used: Instant::now(),
        }
    }
}

/// Concurrency controller for managing invocation limits
pub struct ConcurrencyController {
    /// Configuration
    config: ConcurrencyConfig,

    /// Global semaphore for all invocations
    global_semaphore: Arc<Semaphore>,

    /// Per-function concurrency tracking
    function_limits: Arc<RwLock<HashMap<String, FunctionConcurrency>>>,
}

impl ConcurrencyController {
    /// Create a new concurrency controller
    pub fn new(config: ConcurrencyConfig) -> Self {
        Self {
            global_semaphore: Arc::new(Semaphore::new(config.max_global_concurrent)),
            config,
            function_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Acquire permits for function execution (global + per-function)
    /// Returns a guard that releases permits when dropped
    pub async fn acquire(&self, function_name: &str) -> Result<ConcurrencyGuard, ConcurrencyError> {
        let start_time = Instant::now();

        // Check queue size first (before acquiring any permits)
        {
            let limits = self.function_limits.read().await;
            if let Some(func_limit) = limits.get(function_name)
                && func_limit.stats.current_queued >= self.config.max_queue_size
            {
                return Err(ConcurrencyError::QueueFull);
            }
        }

        // Increment queue counter
        {
            let mut limits = self.function_limits.write().await;
            let func_limit = limits
                .entry(function_name.to_string())
                .or_insert_with(|| FunctionConcurrency::new(self.config.max_per_function));
            func_limit.stats.current_queued += 1;
            func_limit.last_used = Instant::now();
        }

        // Try to acquire global permit with timeout
        let global_permit = match tokio::time::timeout(
            self.config.queue_timeout,
            self.global_semaphore.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) => {
                self.update_stats_timeout(function_name).await;
                return Err(ConcurrencyError::GlobalLimitReached);
            }
            Err(_) => {
                self.update_stats_timeout(function_name).await;
                return Err(ConcurrencyError::Timeout);
            }
        };

        // Get per-function semaphore
        let per_function_semaphore = {
            let limits = self.function_limits.read().await;
            limits
                .get(function_name)
                .map(|f| f.semaphore.clone())
                .unwrap_or_else(|| Arc::new(Semaphore::new(self.config.max_per_function)))
        };

        // Try to acquire per-function permit with timeout
        let function_permit = match tokio::time::timeout(
            self.config.queue_timeout,
            per_function_semaphore.acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) => {
                drop(global_permit); // Release global permit
                self.update_stats_timeout(function_name).await;
                return Err(ConcurrencyError::FunctionLimitReached);
            }
            Err(_) => {
                drop(global_permit); // Release global permit
                self.update_stats_timeout(function_name).await;
                return Err(ConcurrencyError::Timeout);
            }
        };

        // Update statistics
        let queue_wait_ms = start_time.elapsed().as_millis() as f64;
        {
            let mut limits = self.function_limits.write().await;
            if let Some(func_limit) = limits.get_mut(function_name) {
                func_limit.stats.current_queued = func_limit.stats.current_queued.saturating_sub(1);
                func_limit.stats.current_running += 1;
                func_limit.stats.total_processed += 1;

                // Update average queue wait time
                let total_wait = func_limit.stats.avg_queue_wait_ms
                    * (func_limit.stats.total_processed - 1) as f64;
                func_limit.stats.avg_queue_wait_ms =
                    (total_wait + queue_wait_ms) / func_limit.stats.total_processed as f64;
            }
        }

        Ok(ConcurrencyGuard {
            function_name: function_name.to_string(),
            _global_permit: global_permit,
            _function_permit: function_permit,
            controller: self.function_limits.clone(),
        })
    }

    /// Update statistics for timeout
    async fn update_stats_timeout(&self, function_name: &str) {
        let mut limits = self.function_limits.write().await;
        if let Some(func_limit) = limits.get_mut(function_name) {
            func_limit.stats.current_queued = func_limit.stats.current_queued.saturating_sub(1);
            func_limit.stats.total_timeouts += 1;
        }
    }

    /// Get statistics for a function
    pub async fn get_function_stats(
        &self,
        function_name: &str,
    ) -> Option<FunctionConcurrencyStats> {
        let limits = self.function_limits.read().await;
        limits.get(function_name).map(|f| f.stats.clone())
    }

    /// Get statistics for all functions
    pub async fn get_all_stats(&self) -> HashMap<String, FunctionConcurrencyStats> {
        let limits = self.function_limits.read().await;
        limits
            .iter()
            .map(|(name, func)| (name.clone(), func.stats.clone()))
            .collect()
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> GlobalConcurrencyStats {
        GlobalConcurrencyStats {
            max_global_concurrent: self.config.max_global_concurrent,
            current_global_running: self.config.max_global_concurrent
                - self.global_semaphore.available_permits(),
            max_per_function: self.config.max_per_function,
            max_queue_size: self.config.max_queue_size,
        }
    }
}

/// Global concurrency statistics
#[derive(Debug, Clone)]
pub struct GlobalConcurrencyStats {
    pub max_global_concurrent: usize,
    pub current_global_running: usize,
    pub max_per_function: usize,
    pub max_queue_size: usize,
}

/// Guard that releases concurrency permits when dropped
pub struct ConcurrencyGuard {
    function_name: String,
    _global_permit: tokio::sync::OwnedSemaphorePermit,
    _function_permit: tokio::sync::OwnedSemaphorePermit,
    controller: Arc<RwLock<HashMap<String, FunctionConcurrency>>>,
}

impl Drop for ConcurrencyGuard {
    fn drop(&mut self) {
        // Spawn a task to update statistics (since we can't await in Drop)
        let function_name = self.function_name.clone();
        let controller = self.controller.clone();
        tokio::spawn(async move {
            let mut limits = controller.write().await;
            if let Some(func_limit) = limits.get_mut(&function_name) {
                func_limit.stats.current_running =
                    func_limit.stats.current_running.saturating_sub(1);
            }
        });
        // Permits are automatically released when _global_permit and _function_permit are dropped
    }
}

/// Concurrency control errors
#[derive(Debug, thiserror::Error)]
pub enum ConcurrencyError {
    #[error("Global concurrency limit reached")]
    GlobalLimitReached,

    #[error("Function concurrency limit reached")]
    FunctionLimitReached,

    #[error("Request queue is full")]
    QueueFull,

    #[error("Request timed out waiting in queue")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_global_limit() {
        let config = ConcurrencyConfig {
            max_global_concurrent: 2,
            max_per_function: 10,
            ..Default::default()
        };
        let controller = ConcurrencyController::new(config);

        // Acquire 2 permits (should succeed)
        let _guard1 = controller.acquire("test").await.unwrap();
        let _guard2 = controller.acquire("test").await.unwrap();

        // Third should timeout/fail
        let result = controller.acquire("test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_per_function_limit() {
        let config = ConcurrencyConfig {
            max_global_concurrent: 100,
            max_per_function: 2,
            ..Default::default()
        };
        let controller = ConcurrencyController::new(config);

        // Acquire 2 permits for same function (should succeed)
        let _guard1 = controller.acquire("func1").await.unwrap();
        let _guard2 = controller.acquire("func1").await.unwrap();

        // Third for same function should fail
        let result = controller.acquire("func1").await;
        assert!(result.is_err());

        // But different function should succeed
        let _guard3 = controller.acquire("func2").await.unwrap();
    }
}
