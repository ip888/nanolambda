use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// Single metrics data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: i64,
    pub function_name: String,
    pub cold_start: bool,
    pub execution_time_ms: i64,
    pub status: MetricStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MetricStatus {
    Success,
    Error,
    Timeout,
}

impl MetricStatus {
    pub fn as_str(&self) -> &str {
        match self {
            MetricStatus::Success => "success",
            MetricStatus::Error => "error",
            MetricStatus::Timeout => "timeout",
        }
    }
}

/// Aggregated metrics for a time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAggregate {
    pub total_invocations: usize,
    pub cold_starts: usize,
    pub errors: usize,
    pub timeouts: usize,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: i64,
    pub p95_latency_ms: i64,
    pub p99_latency_ms: i64,
    pub invocations_per_second: f64,
    pub cold_start_rate: f64,
    pub error_rate: f64,
}

/// In-memory metrics collector with circular buffer
pub struct MetricsCollector {
    // Store last 1 hour of metrics (max 3600 points if 1/sec)
    points: Arc<RwLock<VecDeque<MetricPoint>>>,
    max_points: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            points: Arc::new(RwLock::new(VecDeque::new())),
            max_points: 10_000, // ~2.7 hours at 1 invocation/sec
        }
    }

    /// Record a new metric point
    pub async fn record(&self, point: MetricPoint) {
        let mut points = self.points.write().await;
        
        // Add new point
        points.push_back(point);
        
        // Remove old points if exceeding max
        while points.len() > self.max_points {
            points.pop_front();
        }
    }

    /// Get metrics for the last N seconds
    pub async fn get_metrics(&self, last_seconds: i64) -> MetricsAggregate {
        let points = self.points.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let cutoff = now - last_seconds;
        
        // Filter points within time window
        let recent_points: Vec<&MetricPoint> = points
            .iter()
            .filter(|p| p.timestamp >= cutoff)
            .collect();
        
        if recent_points.is_empty() {
            return MetricsAggregate {
                total_invocations: 0,
                cold_starts: 0,
                errors: 0,
                timeouts: 0,
                avg_latency_ms: 0.0,
                p50_latency_ms: 0,
                p95_latency_ms: 0,
                p99_latency_ms: 0,
                invocations_per_second: 0.0,
                cold_start_rate: 0.0,
                error_rate: 0.0,
            };
        }
        
        let total = recent_points.len();
        let cold_starts = recent_points.iter().filter(|p| p.cold_start).count();
        let errors = recent_points.iter().filter(|p| p.status == MetricStatus::Error).count();
        let timeouts = recent_points.iter().filter(|p| p.status == MetricStatus::Timeout).count();
        
        // Calculate latency stats
        let mut latencies: Vec<i64> = recent_points.iter().map(|p| p.execution_time_ms).collect();
        latencies.sort_unstable();
        
        let avg_latency = latencies.iter().sum::<i64>() as f64 / latencies.len() as f64;
        let p50 = Self::percentile(&latencies, 50);
        let p95 = Self::percentile(&latencies, 95);
        let p99 = Self::percentile(&latencies, 99);
        
        // Calculate rates
        let duration_seconds = last_seconds as f64;
        let invocations_per_second = total as f64 / duration_seconds;
        let cold_start_rate = cold_starts as f64 / total as f64;
        let error_rate = errors as f64 / total as f64;
        
        MetricsAggregate {
            total_invocations: total,
            cold_starts,
            errors,
            timeouts,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            invocations_per_second,
            cold_start_rate,
            error_rate,
        }
    }

    /// Get all-time metrics
    pub async fn get_all_time_metrics(&self) -> MetricsAggregate {
        let points = self.points.read().await;
        
        if points.is_empty() {
            return MetricsAggregate {
                total_invocations: 0,
                cold_starts: 0,
                errors: 0,
                timeouts: 0,
                avg_latency_ms: 0.0,
                p50_latency_ms: 0,
                p95_latency_ms: 0,
                p99_latency_ms: 0,
                invocations_per_second: 0.0,
                cold_start_rate: 0.0,
                error_rate: 0.0,
            };
        }
        
        let total = points.len();
        let cold_starts = points.iter().filter(|p| p.cold_start).count();
        let errors = points.iter().filter(|p| p.status == MetricStatus::Error).count();
        let timeouts = points.iter().filter(|p| p.status == MetricStatus::Timeout).count();
        
        let mut latencies: Vec<i64> = points.iter().map(|p| p.execution_time_ms).collect();
        latencies.sort_unstable();
        
        let avg_latency = latencies.iter().sum::<i64>() as f64 / latencies.len() as f64;
        let p50 = Self::percentile(&latencies, 50);
        let p95 = Self::percentile(&latencies, 95);
        let p99 = Self::percentile(&latencies, 99);
        
        // Calculate time span
        let first_ts = points.front().unwrap().timestamp;
        let last_ts = points.back().unwrap().timestamp;
        let duration_seconds = (last_ts - first_ts).max(1) as f64;
        
        let invocations_per_second = total as f64 / duration_seconds;
        let cold_start_rate = cold_starts as f64 / total as f64;
        let error_rate = errors as f64 / total as f64;
        
        MetricsAggregate {
            total_invocations: total,
            cold_starts,
            errors,
            timeouts,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            invocations_per_second,
            cold_start_rate,
            error_rate,
        }
    }

    /// Calculate percentile from sorted array
    fn percentile(sorted: &[i64], p: usize) -> i64 {
        if sorted.is_empty() {
            return 0;
        }
        let idx = (sorted.len() * p / 100).min(sorted.len() - 1);
        sorted[idx]
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_metric() {
        let collector = MetricsCollector::new();
        
        let point = MetricPoint {
            timestamp: 1000,
            function_name: "test".to_string(),
            cold_start: true,
            execution_time_ms: 50,
            status: MetricStatus::Success,
        };
        
        collector.record(point).await;
        
        let metrics = collector.get_all_time_metrics().await;
        assert_eq!(metrics.total_invocations, 1);
        assert_eq!(metrics.cold_starts, 1);
    }

    #[tokio::test]
    async fn test_metrics_aggregate() {
        let collector = MetricsCollector::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Record 10 invocations
        for i in 0..10 {
            collector.record(MetricPoint {
                timestamp: now - i,
                function_name: "test".to_string(),
                cold_start: i == 0, // Only first is cold start
                execution_time_ms: 10 + i * 5,
                status: if i < 8 { MetricStatus::Success } else { MetricStatus::Error },
            }).await;
        }
        
        let metrics = collector.get_metrics(60).await;
        assert_eq!(metrics.total_invocations, 10);
        assert_eq!(metrics.cold_starts, 1);
        assert_eq!(metrics.errors, 2);
        assert_eq!(metrics.cold_start_rate, 0.1);
        assert_eq!(metrics.error_rate, 0.2);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let data = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        assert_eq!(MetricsCollector::percentile(&data, 50), 50);
        assert_eq!(MetricsCollector::percentile(&data, 95), 100);
        assert_eq!(MetricsCollector::percentile(&data, 99), 100);
    }
}
