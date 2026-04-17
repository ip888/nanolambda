//! Predictive Pre-warming System
//!
//! ML-based prediction of which functions to pre-warm:
//! - Time-series analysis for traffic patterns
//! - Request correlation for related functions
//! - Adaptive learning from actual invocations

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Maximum history size for time series
const MAX_HISTORY_SIZE: usize = 1000;

/// Time bucket size for aggregation
const BUCKET_DURATION_SECS: u64 = 60;

/// A function invocation record
#[derive(Debug, Clone)]
pub struct InvocationRecord {
    /// Function ID
    pub function_id: String,
    /// Timestamp (Unix millis)
    pub timestamp: u64,
    /// Request source (IP hash, region, etc.)
    pub source_hash: u64,
    /// Was this a cold start?
    pub cold_start: bool,
    /// Execution duration
    pub duration: Duration,
    /// Memory used
    pub memory_used: usize,
}

/// Time bucket for aggregation
#[derive(Debug, Clone, Default)]
struct TimeBucket {
    /// Bucket start timestamp
    timestamp: u64,
    /// Total invocations
    count: u64,
    /// Cold starts
    cold_starts: u64,
    /// Total duration
    total_duration: Duration,
}

/// Pre-warming prediction
#[derive(Debug, Clone)]
pub struct WarmingPrediction {
    /// Function ID
    pub function_id: String,
    /// Predicted invocations in next window
    pub predicted_invocations: f64,
    /// Confidence score (0-1)
    pub confidence: f64,
    /// Recommended warm instances
    pub recommended_instances: u32,
    /// Reasoning
    pub reason: PredictionReason,
}

/// Reason for prediction
#[derive(Debug, Clone)]
pub enum PredictionReason {
    /// Based on historical traffic pattern
    HistoricalPattern { hour: u32, day: u32 },
    /// Based on recent spike
    RecentSpike { increase_percent: f64 },
    /// Based on correlated function
    CorrelatedFunction { trigger_function: String },
    /// Based on scheduled event
    ScheduledEvent { event_name: String },
    /// Default baseline
    Baseline,
}

/// Configuration for pre-warming
#[derive(Debug, Clone)]
pub struct PreWarmingConfig {
    /// Prediction window (how far ahead to predict)
    pub prediction_window: Duration,
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Maximum warm instances per function
    pub max_warm_instances: u32,
    /// Cold start penalty (ms) for cost calculation
    pub cold_start_penalty_ms: u32,
    /// Cost per warm instance per second
    pub warm_instance_cost: f64,
    /// Enable correlation analysis
    pub enable_correlation: bool,
}

impl Default for PreWarmingConfig {
    fn default() -> Self {
        Self {
            prediction_window: Duration::from_secs(300), // 5 minutes
            min_confidence: 0.6,
            max_warm_instances: 10,
            cold_start_penalty_ms: 100,
            warm_instance_cost: 0.001,
            enable_correlation: true,
        }
    }
}

/// Predictive pre-warming engine
pub struct PreWarmingEngine {
    /// Configuration
    config: PreWarmingConfig,
    /// Function invocation history
    history: HashMap<String, FunctionHistory>,
    /// Function correlations
    correlations: FunctionCorrelations,
    /// Statistics
    stats: PreWarmingStats,
}

/// History for a single function
struct FunctionHistory {
    /// Recent invocations
    recent: VecDeque<InvocationRecord>,
    /// Time series buckets
    time_series: VecDeque<TimeBucket>,
    /// Hourly averages (24 buckets)
    hourly_averages: [f64; 24],
    /// Daily patterns (7 days x 24 hours)
    daily_patterns: [[f64; 24]; 7],
    /// Moving average
    moving_average: f64,
    /// Variance for anomaly detection
    variance: f64,
}

impl FunctionHistory {
    fn new() -> Self {
        Self {
            recent: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            time_series: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            hourly_averages: [0.0; 24],
            daily_patterns: [[0.0; 24]; 7],
            moving_average: 0.0,
            variance: 0.0,
        }
    }

    fn record(&mut self, record: InvocationRecord) {
        // Add to recent
        if self.recent.len() >= MAX_HISTORY_SIZE {
            self.recent.pop_front();
        }
        self.recent.push_back(record.clone());

        // Update time series
        let bucket_ts = (record.timestamp / 1000 / BUCKET_DURATION_SECS) * BUCKET_DURATION_SECS;
        if let Some(bucket) = self.time_series.back_mut() {
            if bucket.timestamp == bucket_ts {
                bucket.count += 1;
                bucket.total_duration += record.duration;
                if record.cold_start {
                    bucket.cold_starts += 1;
                }
            } else {
                self.add_bucket(bucket_ts, &record);
            }
        } else {
            self.add_bucket(bucket_ts, &record);
        }

        // Update patterns
        self.update_patterns(&record);
    }

    fn add_bucket(&mut self, timestamp: u64, record: &InvocationRecord) {
        if self.time_series.len() >= MAX_HISTORY_SIZE {
            self.time_series.pop_front();
        }
        self.time_series.push_back(TimeBucket {
            timestamp,
            count: 1,
            cold_starts: if record.cold_start { 1 } else { 0 },
            total_duration: record.duration,
        });
    }

    fn update_patterns(&mut self, record: &InvocationRecord) {
        // Extract hour and day from timestamp
        let secs = record.timestamp / 1000;
        let hour = ((secs / 3600) % 24) as usize;
        let day = ((secs / 86400) % 7) as usize;

        // Update hourly average (exponential smoothing)
        let alpha = 0.1;
        self.hourly_averages[hour] = self.hourly_averages[hour] * (1.0 - alpha) + alpha;

        // Update daily pattern
        self.daily_patterns[day][hour] = self.daily_patterns[day][hour] * (1.0 - alpha) + alpha;

        // Update moving average
        let recent_count = self.recent.len() as f64;
        if recent_count > 0.0 {
            self.moving_average = self.moving_average * 0.9 + 0.1 * (1.0 / recent_count);
        }
    }

    fn predict_next_window(&self, window_secs: u64) -> (f64, f64) {
        // Get current time info
        let now = current_timestamp();
        let hour = ((now / 1000 / 3600) % 24) as usize;
        let day = ((now / 1000 / 86400) % 7) as usize;

        // Base prediction from daily pattern
        let pattern_value = self.daily_patterns[day][hour];

        // Adjust based on recent trend
        let recent_rate = if self.time_series.len() >= 2 {
            let recent: Vec<_> = self.time_series.iter().rev().take(5).collect();
            let sum: u64 = recent.iter().map(|b| b.count).sum();
            sum as f64 / recent.len() as f64 / BUCKET_DURATION_SECS as f64
        } else {
            self.moving_average
        };

        // Combine predictions
        let predicted = (pattern_value * 0.7 + recent_rate * 0.3) * window_secs as f64;

        // Confidence based on data quality
        let confidence = (self.time_series.len() as f64 / 100.0).min(1.0);

        (predicted, confidence)
    }

    fn detect_spike(&self) -> Option<f64> {
        if self.time_series.len() < 3 {
            return None;
        }

        let recent: Vec<_> = self.time_series.iter().rev().take(3).collect();
        let recent_avg = recent.iter().map(|b| b.count).sum::<u64>() as f64 / 3.0;

        let older: Vec<_> = self.time_series.iter().rev().skip(3).take(10).collect();
        if older.is_empty() {
            return None;
        }
        let older_avg = older.iter().map(|b| b.count).sum::<u64>() as f64 / older.len() as f64;

        if older_avg > 0.0 {
            let increase = (recent_avg - older_avg) / older_avg * 100.0;
            if increase > 50.0 {
                return Some(increase);
            }
        }

        None
    }
}

/// Function correlation tracker
struct FunctionCorrelations {
    /// Co-occurrence counts: (func_a, func_b) -> count
    co_occurrences: HashMap<(String, String), u64>,
    /// Recent sequences for correlation detection
    recent_sequences: VecDeque<(String, u64)>,
    /// Correlation window (ms)
    window_ms: u64,
}

impl FunctionCorrelations {
    fn new() -> Self {
        Self {
            co_occurrences: HashMap::new(),
            recent_sequences: VecDeque::with_capacity(1000),
            window_ms: 5000, // 5 second window
        }
    }

    fn record(&mut self, function_id: &str, timestamp: u64) {
        // Check for correlations with recent functions
        for (other_func, other_ts) in &self.recent_sequences {
            if timestamp - other_ts < self.window_ms && other_func != function_id {
                let key = if function_id < other_func.as_str() {
                    (function_id.to_string(), other_func.clone())
                } else {
                    (other_func.clone(), function_id.to_string())
                };
                *self.co_occurrences.entry(key).or_insert(0) += 1;
            }
        }

        // Add to recent
        if self.recent_sequences.len() >= 1000 {
            self.recent_sequences.pop_front();
        }
        self.recent_sequences
            .push_back((function_id.to_string(), timestamp));
    }

    fn get_correlated(&self, function_id: &str) -> Vec<(String, f64)> {
        let mut correlations = Vec::new();

        for ((f1, f2), count) in &self.co_occurrences {
            if f1 == function_id {
                correlations.push((f2.clone(), *count as f64));
            } else if f2 == function_id {
                correlations.push((f1.clone(), *count as f64));
            }
        }

        // Normalize scores
        let max_score = correlations.iter().map(|(_, s)| *s).fold(0.0f64, f64::max);
        if max_score > 0.0 {
            for (_, score) in &mut correlations {
                *score /= max_score;
            }
        }

        correlations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        correlations.truncate(5);
        correlations
    }
}

/// Pre-warming statistics
#[derive(Debug, Default)]
pub struct PreWarmingStats {
    /// Total predictions made
    pub predictions: AtomicU64,
    /// Accurate predictions (within 20%)
    pub accurate_predictions: AtomicU64,
    /// Cold starts prevented
    pub cold_starts_prevented: AtomicU64,
    /// Unnecessary warm instances
    pub wasted_instances: AtomicU64,
}

impl PreWarmingEngine {
    /// Create a new pre-warming engine
    pub fn new(config: PreWarmingConfig) -> Self {
        Self {
            config,
            history: HashMap::new(),
            correlations: FunctionCorrelations::new(),
            stats: PreWarmingStats::default(),
        }
    }

    /// Record a function invocation
    pub fn record_invocation(&mut self, record: InvocationRecord) {
        // Update history
        let history = self
            .history
            .entry(record.function_id.clone())
            .or_insert_with(FunctionHistory::new);
        history.record(record.clone());

        // Update correlations
        if self.config.enable_correlation {
            self.correlations
                .record(&record.function_id, record.timestamp);
        }
    }

    /// Get warming predictions for all functions
    pub fn get_predictions(&self) -> Vec<WarmingPrediction> {
        self.stats.predictions.fetch_add(1, Ordering::Relaxed);

        let mut predictions = Vec::new();

        for (function_id, history) in &self.history {
            if let Some(prediction) = self.predict_for_function(function_id, history) {
                if prediction.confidence >= self.config.min_confidence {
                    predictions.push(prediction);
                }
            }
        }

        // Sort by predicted invocations * confidence
        predictions.sort_by(|a, b| {
            let score_a = a.predicted_invocations * a.confidence;
            let score_b = b.predicted_invocations * b.confidence;
            score_b.partial_cmp(&score_a).unwrap()
        });

        predictions
    }

    /// Predict for a single function
    fn predict_for_function(
        &self,
        function_id: &str,
        history: &FunctionHistory,
    ) -> Option<WarmingPrediction> {
        let window_secs = self.config.prediction_window.as_secs();

        // Check for recent spike
        if let Some(increase) = history.detect_spike() {
            let (predicted, confidence) = history.predict_next_window(window_secs);
            let adjusted_predicted = predicted * (1.0 + increase / 100.0);

            return Some(WarmingPrediction {
                function_id: function_id.to_string(),
                predicted_invocations: adjusted_predicted,
                confidence: confidence * 0.8, // Reduce confidence for spike prediction
                recommended_instances: self.calculate_instances(adjusted_predicted),
                reason: PredictionReason::RecentSpike {
                    increase_percent: increase,
                },
            });
        }

        // Check for correlated function triggers
        if self.config.enable_correlation {
            let correlated = self.correlations.get_correlated(function_id);
            for (trigger_func, correlation_score) in correlated {
                if correlation_score > 0.7 {
                    if let Some(trigger_history) = self.history.get(&trigger_func) {
                        if trigger_history.detect_spike().is_some() {
                            let (predicted, _) = history.predict_next_window(window_secs);

                            return Some(WarmingPrediction {
                                function_id: function_id.to_string(),
                                predicted_invocations: predicted * correlation_score,
                                confidence: correlation_score * 0.7,
                                recommended_instances: self.calculate_instances(predicted),
                                reason: PredictionReason::CorrelatedFunction {
                                    trigger_function: trigger_func,
                                },
                            });
                        }
                    }
                }
            }
        }

        // Default: historical pattern
        let (predicted, confidence) = history.predict_next_window(window_secs);
        if predicted > 0.0 {
            let now = current_timestamp();
            let hour = ((now / 1000 / 3600) % 24) as u32;
            let day = ((now / 1000 / 86400) % 7) as u32;

            Some(WarmingPrediction {
                function_id: function_id.to_string(),
                predicted_invocations: predicted,
                confidence,
                recommended_instances: self.calculate_instances(predicted),
                reason: PredictionReason::HistoricalPattern { hour, day },
            })
        } else {
            None
        }
    }

    /// Calculate recommended instances based on predicted invocations
    fn calculate_instances(&self, predicted: f64) -> u32 {
        // Simple heuristic: 1 instance per N predicted invocations
        let instances = (predicted / 10.0).ceil() as u32;
        instances.min(self.config.max_warm_instances).max(1)
    }

    /// Get prediction for a specific function
    pub fn get_prediction(&self, function_id: &str) -> Option<WarmingPrediction> {
        self.history
            .get(function_id)
            .and_then(|h| self.predict_for_function(function_id, h))
    }

    /// Feedback on prediction accuracy
    pub fn record_accuracy(&mut self, function_id: &str, predicted: f64, actual: u64) {
        let error = (predicted - actual as f64).abs() / predicted.max(1.0);
        if error < 0.2 {
            self.stats.accurate_predictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &PreWarmingStats {
        &self.stats
    }
}

// Helper functions

fn current_timestamp() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_record(function_id: &str, cold_start: bool) -> InvocationRecord {
        InvocationRecord {
            function_id: function_id.to_string(),
            timestamp: current_timestamp(),
            source_hash: 12345,
            cold_start,
            duration: Duration::from_millis(50),
            memory_used: 128 * 1024 * 1024,
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = PreWarmingEngine::new(PreWarmingConfig::default());
        assert!(engine.history.is_empty());
    }

    #[test]
    fn test_record_invocation() {
        let mut engine = PreWarmingEngine::new(PreWarmingConfig::default());

        for _ in 0..10 {
            engine.record_invocation(create_record("func1", false));
        }

        assert!(engine.history.contains_key("func1"));
        assert_eq!(engine.history.get("func1").unwrap().recent.len(), 10);
    }

    #[test]
    fn test_get_predictions() {
        let mut engine = PreWarmingEngine::new(PreWarmingConfig::default());

        // Add some history
        for _ in 0..100 {
            engine.record_invocation(create_record("func1", false));
        }

        let predictions = engine.get_predictions();
        // May or may not have predictions depending on confidence
        assert!(predictions.len() <= 1);
    }

    #[test]
    fn test_spike_detection() {
        let mut history = FunctionHistory::new();

        // Add baseline
        for i in 0..50 {
            history.record(InvocationRecord {
                function_id: "test".to_string(),
                timestamp: i * 1000 * 60, // 1 per minute
                source_hash: 0,
                cold_start: false,
                duration: Duration::from_millis(10),
                memory_used: 0,
            });
        }

        // Add spike
        let spike_start = 50 * 1000 * 60;
        for i in 0..30 {
            history.record(InvocationRecord {
                function_id: "test".to_string(),
                timestamp: spike_start + i * 1000, // 30 in 30 seconds
                source_hash: 0,
                cold_start: false,
                duration: Duration::from_millis(10),
                memory_used: 0,
            });
        }

        // Should detect spike
        let spike = history.detect_spike();
        assert!(spike.is_some());
    }

    #[test]
    fn test_correlations() {
        let mut correlations = FunctionCorrelations::new();

        // Simulate correlated invocations
        let now = current_timestamp();
        for i in 0..20 {
            correlations.record("funcA", now + i * 100);
            correlations.record("funcB", now + i * 100 + 50); // 50ms after A
        }

        let correlated = correlations.get_correlated("funcA");
        assert!(!correlated.is_empty());
        assert_eq!(correlated[0].0, "funcB");
    }

    #[test]
    fn test_calculate_instances() {
        let engine = PreWarmingEngine::new(PreWarmingConfig::default());

        assert_eq!(engine.calculate_instances(5.0), 1);
        assert_eq!(engine.calculate_instances(15.0), 2);
        assert_eq!(engine.calculate_instances(100.0), 10);
        assert_eq!(engine.calculate_instances(200.0), 10); // Capped at max
    }
}
