//! Analytics Engine Integration for NanoLambda Edge
//!
//! Cloudflare Analytics Engine provides a high-performance, SQL-queryable
//! data warehouse for real-time analytics at the edge.
//!
//! # Features
//!
//! - **Real-time Ingestion**: Write events with sub-millisecond latency
//! - **SQL Queries**: Query data using familiar SQL syntax
//! - **Automatic Aggregation**: Built-in time-series aggregations
//! - **Zero Infrastructure**: Fully managed by Cloudflare
//!
//! # Data Model
//!
//! Events consist of:
//! - `blob1-20`: Up to 20 string fields (indexed)
//! - `double1-20`: Up to 20 numeric fields (aggregatable)
//! - `_sample_interval`: Sampling rate (1 = 100%)
//!
//! # Example Usage
//!
//! ```rust,ignore
//! let analytics = AnalyticsEngine::new(env)?;
//! 
//! // Write an event
//! analytics.write_event(AnalyticsEvent {
//!     blob1: Some("function_invoke".to_string()),
//!     blob2: Some(function_id),
//!     double1: Some(duration_ms),
//!     ..Default::default()
//! }).await?;
//!
//! // Query data
//! let results = analytics.query(
//!     "SELECT blob2 as function_id, SUM(double1) as total_ms 
//!      FROM events WHERE blob1 = 'function_invoke' 
//!      GROUP BY blob2"
//! ).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::{Env, Result as WorkerResult};

/// Analytics event structure
/// 
/// Maps to Cloudflare Analytics Engine schema:
/// - blob1-20: String fields for dimensions/labels
/// - double1-20: Numeric fields for metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    // String fields (dimensions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob1: Option<String>,  // event_type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob2: Option<String>,  // function_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob3: Option<String>,  // user_id / api_key_hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob4: Option<String>,  // region / colo
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob5: Option<String>,  // status (success/error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob6: Option<String>,  // runtime (js/python/wasm)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob7: Option<String>,  // version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob8: Option<String>,  // error_type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob9: Option<String>,  // country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob10: Option<String>, // path
    
    // Numeric fields (metrics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double1: Option<f64>,   // duration_ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double2: Option<f64>,   // memory_mb
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double3: Option<f64>,   // cpu_time_ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double4: Option<f64>,   // response_bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double5: Option<f64>,   // request_bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double6: Option<f64>,   // cold_start (1.0 or 0.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double7: Option<f64>,   // error_count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double8: Option<f64>,   // retry_count
    
    /// Sampling interval (1 = no sampling, 10 = 1/10 sampled)
    #[serde(rename = "_sample_interval", default = "default_sample")]
    pub sample_interval: u32,
}

fn default_sample() -> u32 { 1 }

/// Pre-defined event types
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    /// Function invocation
    FunctionInvoke,
    /// Function cold start
    FunctionColdStart,
    /// API request
    ApiRequest,
    /// Error occurred
    Error,
    /// Vector operation
    VectorOp,
    /// AI inference
    AiInference,
    /// Cache hit
    CacheHit,
    /// Cache miss
    CacheMiss,
    /// Rate limit exceeded
    RateLimited,
    /// Authentication event
    AuthEvent,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::FunctionInvoke => "function_invoke",
            EventType::FunctionColdStart => "function_cold_start",
            EventType::ApiRequest => "api_request",
            EventType::Error => "error",
            EventType::VectorOp => "vector_op",
            EventType::AiInference => "ai_inference",
            EventType::CacheHit => "cache_hit",
            EventType::CacheMiss => "cache_miss",
            EventType::RateLimited => "rate_limited",
            EventType::AuthEvent => "auth_event",
        }
    }
}

/// Analytics Engine client
pub struct AnalyticsEngine {
    /// Environment binding name
    binding_name: String,
}

impl AnalyticsEngine {
    /// Create a new Analytics Engine client
    pub fn new(binding_name: impl Into<String>) -> Self {
        Self {
            binding_name: binding_name.into(),
        }
    }

    /// Default binding name
    pub fn default_binding() -> Self {
        Self::new("ANALYTICS")
    }

    /// Write a single event to Analytics Engine
    /// 
    /// Events are written asynchronously and may be batched by Cloudflare.
    pub async fn write_event(&self, env: &Env, event: AnalyticsEvent) -> WorkerResult<()> {
        // In production, this would use the Analytics Engine binding
        // For now, we log the event for development
        worker::console_log!(
            "Analytics Event: type={} function={} duration={}ms",
            event.blob1.as_deref().unwrap_or("unknown"),
            event.blob2.as_deref().unwrap_or("unknown"),
            event.double1.unwrap_or(0.0)
        );
        
        // Actual implementation would be:
        // let analytics = env.analytics(&self.binding_name)?;
        // analytics.write_data_point(event)?;
        
        Ok(())
    }

    /// Write a batch of events
    pub async fn write_batch(&self, env: &Env, events: Vec<AnalyticsEvent>) -> WorkerResult<()> {
        for event in events {
            self.write_event(env, event).await?;
        }
        Ok(())
    }

    /// Helper: Track function invocation
    pub async fn track_invocation(
        &self,
        env: &Env,
        function_id: &str,
        user_id: &str,
        duration_ms: f64,
        memory_mb: f64,
        success: bool,
        cold_start: bool,
        region: Option<&str>,
    ) -> WorkerResult<()> {
        let event = AnalyticsEvent {
            blob1: Some(EventType::FunctionInvoke.as_str().to_string()),
            blob2: Some(function_id.to_string()),
            blob3: Some(user_id.to_string()),
            blob4: region.map(String::from),
            blob5: Some(if success { "success" } else { "error" }.to_string()),
            double1: Some(duration_ms),
            double2: Some(memory_mb),
            double6: Some(if cold_start { 1.0 } else { 0.0 }),
            ..Default::default()
        };
        self.write_event(env, event).await
    }

    /// Helper: Track API request
    pub async fn track_api_request(
        &self,
        env: &Env,
        path: &str,
        method: &str,
        status: u16,
        duration_ms: f64,
        user_id: Option<&str>,
        region: Option<&str>,
    ) -> WorkerResult<()> {
        let event = AnalyticsEvent {
            blob1: Some(EventType::ApiRequest.as_str().to_string()),
            blob2: Some(path.to_string()),
            blob3: user_id.map(String::from),
            blob4: region.map(String::from),
            blob5: Some(status.to_string()),
            blob6: Some(method.to_string()),
            double1: Some(duration_ms),
            ..Default::default()
        };
        self.write_event(env, event).await
    }

    /// Helper: Track error
    pub async fn track_error(
        &self,
        env: &Env,
        error_type: &str,
        message: &str,
        function_id: Option<&str>,
        user_id: Option<&str>,
    ) -> WorkerResult<()> {
        let event = AnalyticsEvent {
            blob1: Some(EventType::Error.as_str().to_string()),
            blob2: function_id.map(String::from),
            blob3: user_id.map(String::from),
            blob8: Some(error_type.to_string()),
            blob10: Some(message.chars().take(256).collect()),
            double7: Some(1.0),
            ..Default::default()
        };
        self.write_event(env, event).await
    }

    /// Helper: Track AI inference
    pub async fn track_ai_inference(
        &self,
        env: &Env,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
        duration_ms: f64,
        user_id: &str,
    ) -> WorkerResult<()> {
        let event = AnalyticsEvent {
            blob1: Some(EventType::AiInference.as_str().to_string()),
            blob2: Some(model.to_string()),
            blob3: Some(user_id.to_string()),
            double1: Some(duration_ms),
            double4: Some(output_tokens as f64),
            double5: Some(input_tokens as f64),
            ..Default::default()
        };
        self.write_event(env, event).await
    }

    /// Helper: Track cache operation
    pub async fn track_cache(
        &self,
        env: &Env,
        cache_key: &str,
        hit: bool,
        size_bytes: Option<u64>,
    ) -> WorkerResult<()> {
        let event_type = if hit { EventType::CacheHit } else { EventType::CacheMiss };
        let event = AnalyticsEvent {
            blob1: Some(event_type.as_str().to_string()),
            blob2: Some(cache_key.to_string()),
            double4: size_bytes.map(|s| s as f64),
            ..Default::default()
        };
        self.write_event(env, event).await
    }
}

/// Query result from Analytics Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column names
    pub columns: Vec<String>,
    /// Row data
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Total rows returned
    pub row_count: usize,
    /// Query execution time in ms
    pub execution_time_ms: f64,
}

/// Analytics query builder for common queries
pub struct AnalyticsQueryBuilder {
    select: Vec<String>,
    from: String,
    where_clauses: Vec<String>,
    group_by: Vec<String>,
    order_by: Option<(String, bool)>, // (column, descending)
    limit: Option<u32>,
    time_range: Option<(String, String)>,
}

impl AnalyticsQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            select: Vec::new(),
            from: "events".to_string(),
            where_clauses: Vec::new(),
            group_by: Vec::new(),
            order_by: None,
            limit: None,
            time_range: None,
        }
    }

    /// Select columns
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select.extend(columns.iter().map(|s| s.to_string()));
        self
    }

    /// Add WHERE clause
    pub fn where_eq(mut self, column: &str, value: &str) -> Self {
        self.where_clauses.push(format!("{} = '{}'", column, value.replace('\'', "''")));
        self
    }

    /// Add WHERE clause for event type
    pub fn event_type(self, event_type: EventType) -> Self {
        self.where_eq("blob1", event_type.as_str())
    }

    /// Add GROUP BY
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by.extend(columns.iter().map(|s| s.to_string()));
        self
    }

    /// Add ORDER BY
    pub fn order_by(mut self, column: &str, descending: bool) -> Self {
        self.order_by = Some((column.to_string(), descending));
        self
    }

    /// Set LIMIT
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set time range (ISO 8601 format)
    pub fn time_range(mut self, start: &str, end: &str) -> Self {
        self.time_range = Some((start.to_string(), end.to_string()));
        self
    }

    /// Build the SQL query
    pub fn build(&self) -> String {
        let mut query = String::new();
        
        // SELECT
        query.push_str("SELECT ");
        if self.select.is_empty() {
            query.push('*');
        } else {
            query.push_str(&self.select.join(", "));
        }
        
        // FROM
        query.push_str(" FROM ");
        query.push_str(&self.from);
        
        // WHERE
        let mut conditions = self.where_clauses.clone();
        if let Some((start, end)) = &self.time_range {
            conditions.push(format!("timestamp >= '{}'", start));
            conditions.push(format!("timestamp <= '{}'", end));
        }
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }
        
        // GROUP BY
        if !self.group_by.is_empty() {
            query.push_str(" GROUP BY ");
            query.push_str(&self.group_by.join(", "));
        }
        
        // ORDER BY
        if let Some((column, desc)) = &self.order_by {
            query.push_str(" ORDER BY ");
            query.push_str(column);
            if *desc {
                query.push_str(" DESC");
            }
        }
        
        // LIMIT
        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        query
    }
}

impl Default for AnalyticsQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-built analytics queries
pub struct AnalyticsQueries;

impl AnalyticsQueries {
    /// Get function invocation counts by function
    pub fn invocations_by_function(limit: u32) -> String {
        AnalyticsQueryBuilder::new()
            .select(&["blob2 as function_id", "COUNT(*) as invocations", "AVG(double1) as avg_duration_ms"])
            .event_type(EventType::FunctionInvoke)
            .group_by(&["blob2"])
            .order_by("invocations", true)
            .limit(limit)
            .build()
    }

    /// Get error rates by function
    pub fn error_rates_by_function(limit: u32) -> String {
        AnalyticsQueryBuilder::new()
            .select(&[
                "blob2 as function_id",
                "COUNT(*) as total",
                "SUM(CASE WHEN blob5 = 'error' THEN 1 ELSE 0 END) as errors",
                "SUM(CASE WHEN blob5 = 'error' THEN 1 ELSE 0 END) * 100.0 / COUNT(*) as error_rate"
            ])
            .event_type(EventType::FunctionInvoke)
            .group_by(&["blob2"])
            .order_by("error_rate", true)
            .limit(limit)
            .build()
    }

    /// Get P50/P95/P99 latency
    pub fn latency_percentiles() -> String {
        AnalyticsQueryBuilder::new()
            .select(&[
                "PERCENTILE_CONT(double1, 0.5) as p50_ms",
                "PERCENTILE_CONT(double1, 0.95) as p95_ms",
                "PERCENTILE_CONT(double1, 0.99) as p99_ms"
            ])
            .event_type(EventType::FunctionInvoke)
            .build()
    }

    /// Get cold start percentage
    pub fn cold_start_rate() -> String {
        AnalyticsQueryBuilder::new()
            .select(&[
                "COUNT(*) as total",
                "SUM(double6) as cold_starts",
                "SUM(double6) * 100.0 / COUNT(*) as cold_start_rate"
            ])
            .event_type(EventType::FunctionInvoke)
            .build()
    }

    /// Get requests by region
    pub fn requests_by_region(limit: u32) -> String {
        AnalyticsQueryBuilder::new()
            .select(&["blob4 as region", "COUNT(*) as requests"])
            .event_type(EventType::ApiRequest)
            .group_by(&["blob4"])
            .order_by("requests", true)
            .limit(limit)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_strings() {
        assert_eq!(EventType::FunctionInvoke.as_str(), "function_invoke");
        assert_eq!(EventType::CacheHit.as_str(), "cache_hit");
    }

    #[test]
    fn test_query_builder() {
        let query = AnalyticsQueryBuilder::new()
            .select(&["blob1", "COUNT(*)"])
            .where_eq("blob1", "test")
            .group_by(&["blob1"])
            .limit(10)
            .build();
        
        assert!(query.contains("SELECT blob1, COUNT(*)"));
        assert!(query.contains("WHERE blob1 = 'test'"));
        assert!(query.contains("GROUP BY blob1"));
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_invocations_query() {
        let query = AnalyticsQueries::invocations_by_function(10);
        assert!(query.contains("function_invoke"));
        assert!(query.contains("GROUP BY"));
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_analytics_event_serialization() {
        let event = AnalyticsEvent {
            blob1: Some("test".to_string()),
            double1: Some(42.5),
            ..Default::default()
        };
        
        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("\"blob1\":\"test\""));
        assert!(json.contains("\"double1\":42.5"));
    }
}
