//! Usage tracking for billing and analytics

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Usage record for a single invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub timestamp: i64,
    pub api_key: String,
    pub function_name: String,
    pub execution_time_ms: i64,
    pub memory_used_mb: f64,
    pub cold_start: bool,
    pub success: bool,
}

/// Aggregated usage statistics for an API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub api_key: String,
    pub total_invocations: u64,
    pub successful_invocations: u64,
    pub failed_invocations: u64,
    pub total_execution_time_ms: u64,
    pub total_memory_mb_seconds: f64,
    pub cold_starts: u64,
    pub functions_used: Vec<String>,
    pub first_use: i64,
    pub last_use: i64,
}

impl UsageStats {
    fn new(api_key: String, timestamp: i64) -> Self {
        Self {
            api_key,
            total_invocations: 0,
            successful_invocations: 0,
            failed_invocations: 0,
            total_execution_time_ms: 0,
            total_memory_mb_seconds: 0.0,
            cold_starts: 0,
            functions_used: Vec::new(),
            first_use: timestamp,
            last_use: timestamp,
        }
    }

    fn record(&mut self, usage: &UsageRecord) {
        self.total_invocations += 1;
        
        if usage.success {
            self.successful_invocations += 1;
        } else {
            self.failed_invocations += 1;
        }
        
        self.total_execution_time_ms += usage.execution_time_ms as u64;
        
        // Memory MB-seconds: (memory_mb * execution_seconds)
        let execution_seconds = usage.execution_time_ms as f64 / 1000.0;
        self.total_memory_mb_seconds += usage.memory_used_mb * execution_seconds;
        
        if usage.cold_start {
            self.cold_starts += 1;
        }
        
        if !self.functions_used.contains(&usage.function_name) {
            self.functions_used.push(usage.function_name.clone());
        }
        
        self.last_use = usage.timestamp;
    }
}

/// Usage tracker for billing
pub struct UsageTracker {
    /// Usage stats aggregated by API key
    stats: Arc<RwLock<HashMap<String, UsageStats>>>,
    
    /// Detailed usage records (circular buffer, last N records)
    records: Arc<RwLock<Vec<UsageRecord>>>,
    
    /// Maximum records to keep in memory
    max_records: usize,
}

impl UsageTracker {
    pub fn new(max_records: usize) -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
            records: Arc::new(RwLock::new(Vec::with_capacity(max_records))),
            max_records,
        }
    }

    /// Record usage for an invocation
    pub async fn record(&self, usage: UsageRecord) {
        // Update aggregated stats
        {
            let mut stats = self.stats.write().await;
            let stat = stats
                .entry(usage.api_key.clone())
                .or_insert_with(|| UsageStats::new(usage.api_key.clone(), usage.timestamp));
            stat.record(&usage);
        }

        // Store detailed record
        {
            let mut records = self.records.write().await;
            records.push(usage);
            
            // Keep only last N records
            if records.len() > self.max_records {
                records.remove(0);
            }
        }
    }

    /// Get usage stats for a specific API key
    pub async fn get_stats(&self, api_key: &str) -> Option<UsageStats> {
        let stats = self.stats.read().await;
        stats.get(api_key).cloned()
    }

    /// Get usage stats for all API keys
    pub async fn get_all_stats(&self) -> Vec<UsageStats> {
        let stats = self.stats.read().await;
        stats.values().cloned().collect()
    }

    /// Get detailed records for an API key
    pub async fn get_records(&self, api_key: &str, limit: usize) -> Vec<UsageRecord> {
        let records = self.records.read().await;
        records
            .iter()
            .filter(|r| r.api_key == api_key)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get usage stats for a specific time window
    pub async fn get_stats_since(&self, api_key: &str, since_timestamp: i64) -> UsageStats {
        let records = self.records.read().await;
        
        let mut stats = UsageStats::new(api_key.to_string(), since_timestamp);
        
        for record in records.iter() {
            if record.api_key == api_key && record.timestamp >= since_timestamp {
                stats.record(record);
            }
        }
        
        stats
    }

    /// Calculate billing amount based on usage
    /// Pricing model: $0.0001 per invocation + $0.00001 per GB-second
    pub fn calculate_bill(stats: &UsageStats) -> BillingInfo {
        // Base price per invocation
        let invocation_cost = stats.total_invocations as f64 * 0.0001;
        
        // Memory cost (convert MB-seconds to GB-seconds)
        let gb_seconds = stats.total_memory_mb_seconds / 1024.0;
        let memory_cost = gb_seconds * 0.00001;
        
        // Total cost
        let total_cost = invocation_cost + memory_cost;
        
        BillingInfo {
            api_key: stats.api_key.clone(),
            invocations: stats.total_invocations,
            successful_invocations: stats.successful_invocations,
            failed_invocations: stats.failed_invocations,
            total_execution_time_ms: stats.total_execution_time_ms,
            gb_seconds,
            invocation_cost,
            memory_cost,
            total_cost,
            period_start: stats.first_use,
            period_end: stats.last_use,
        }
    }

    /// Reset stats for an API key (after billing period)
    pub async fn reset_stats(&self, api_key: &str) {
        let mut stats = self.stats.write().await;
        stats.remove(api_key);
    }

    /// Clear all records older than timestamp
    pub async fn clear_old_records(&self, before_timestamp: i64) {
        let mut records = self.records.write().await;
        records.retain(|r| r.timestamp >= before_timestamp);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BillingInfo {
    pub api_key: String,
    pub invocations: u64,
    pub successful_invocations: u64,
    pub failed_invocations: u64,
    pub total_execution_time_ms: u64,
    pub gb_seconds: f64,
    pub invocation_cost: f64,
    pub memory_cost: f64,
    pub total_cost: f64,
    pub period_start: i64,
    pub period_end: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_usage_tracking() {
        let tracker = UsageTracker::new(1000);
        
        let usage = UsageRecord {
            timestamp: 1234567890,
            api_key: "test_key".to_string(),
            function_name: "test_fn".to_string(),
            execution_time_ms: 100,
            memory_used_mb: 128.0,
            cold_start: true,
            success: true,
        };
        
        tracker.record(usage).await;
        
        let stats = tracker.get_stats("test_key").await.unwrap();
        assert_eq!(stats.total_invocations, 1);
        assert_eq!(stats.successful_invocations, 1);
        assert_eq!(stats.cold_starts, 1);
    }

    #[test]
    fn test_billing_calculation() {
        let stats = UsageStats {
            api_key: "test".to_string(),
            total_invocations: 10000,
            successful_invocations: 9900,
            failed_invocations: 100,
            total_execution_time_ms: 100000,
            total_memory_mb_seconds: 128000.0, // 125 GB-seconds
            cold_starts: 100,
            functions_used: vec!["test_fn".to_string()],
            first_use: 0,
            last_use: 1000,
        };
        
        let bill = UsageTracker::calculate_bill(&stats);
        
        // 10000 invocations * $0.0001 = $1.00
        assert!((bill.invocation_cost - 1.0).abs() < 0.01);
        
        // 125 GB-seconds * $0.00001 = $0.00125
        assert!((bill.memory_cost - 0.00125).abs() < 0.0001);
        
        // Total: ~$1.00
        assert!((bill.total_cost - 1.00125).abs() < 0.01);
    }
}
