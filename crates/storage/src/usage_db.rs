//! Persistent usage tracking with SQLite
//!
//! This module provides append-only usage event logging and aggregated statistics
//! for billing, analytics, and audit trails.

use sqlx::{Pool, Sqlite, Row};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Individual usage event (append-only log)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub api_key: String,
    pub function_name: String,
    pub request_id: String,
    pub execution_time_ms: i64,
    pub memory_mb: u32,
    pub cold_start: bool,
    pub success: bool,
    pub invocation_cost: f64,
    pub compute_cost: f64,
    pub total_cost: f64,
}

/// Aggregated usage statistics (daily rollups)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub api_key: String,
    pub date: String, // YYYY-MM-DD
    pub total_invocations: u64,
    pub successful_invocations: u64,
    pub failed_invocations: u64,
    pub total_execution_time_ms: u64,
    pub total_memory_mb_seconds: f64,
    pub cold_starts: u64,
    pub total_cost: f64,
    pub functions_used: String, // JSON array
}

/// Billing information for an API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingInfo {
    pub api_key: String,
    pub period_start: i64,
    pub period_end: i64,
    pub total_invocations: u64,
    pub total_cost: f64,
    pub invocation_cost: f64,
    pub compute_cost: f64,
    pub breakdown: Vec<DailyCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    pub date: String,
    pub invocations: u64,
    pub cost: f64,
}

/// Usage database manager
pub struct UsageDb {
    pool: Pool<Sqlite>,
    cache: Arc<RwLock<HashMap<String, CachedStats>>>,
}

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct CachedStats {
    stats: UsageStats,
    last_updated: i64,
}

impl UsageDb {
    /// Create new usage database manager
    pub async fn new(pool: Pool<Sqlite>) -> Result<Self, sqlx::Error> {
        let db = Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        db.init_tables().await?;
        Ok(db)
    }
    
    /// Initialize database tables
    async fn init_tables(&self) -> Result<(), sqlx::Error> {
        // Usage events table (append-only)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                api_key TEXT NOT NULL,
                function_name TEXT NOT NULL,
                request_id TEXT NOT NULL,
                execution_time_ms INTEGER NOT NULL,
                memory_mb INTEGER NOT NULL,
                cold_start BOOLEAN NOT NULL,
                success BOOLEAN NOT NULL,
                invocation_cost REAL NOT NULL,
                compute_cost REAL NOT NULL,
                total_cost REAL NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Index for fast queries by API key and time range
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_usage_events_api_key_timestamp 
             ON usage_events(api_key, timestamp)"
        )
        .execute(&self.pool)
        .await?;
        
        // Daily aggregated statistics
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_stats (
                api_key TEXT NOT NULL,
                date TEXT NOT NULL,
                total_invocations INTEGER NOT NULL DEFAULT 0,
                successful_invocations INTEGER NOT NULL DEFAULT 0,
                failed_invocations INTEGER NOT NULL DEFAULT 0,
                total_execution_time_ms INTEGER NOT NULL DEFAULT 0,
                total_memory_mb_seconds REAL NOT NULL DEFAULT 0.0,
                cold_starts INTEGER NOT NULL DEFAULT 0,
                total_cost REAL NOT NULL DEFAULT 0.0,
                functions_used TEXT NOT NULL DEFAULT '[]',
                PRIMARY KEY (api_key, date)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Usage alerts tracking table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                api_key TEXT NOT NULL,
                alert_type TEXT NOT NULL,
                threshold_percent INTEGER NOT NULL,
                current_usage INTEGER NOT NULL,
                usage_limit INTEGER NOT NULL,
                sent_at INTEGER NOT NULL,
                period_start INTEGER NOT NULL,
                period_end INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Index for checking if alert was already sent for this period
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_usage_alerts_api_key_type_period 
             ON usage_alerts(api_key, alert_type, period_start)"
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Record a usage event (async, non-blocking)
    pub fn record_event(&self, event: UsageEvent) {
        let pool = self.pool.clone();
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::insert_event(&pool, &event).await {
                tracing::error!("Failed to record usage event: {:?}", e);
            } else {
                // Update cache asynchronously
                if let Err(e) = Self::update_cache(&pool, &cache, &event.api_key).await {
                    tracing::error!("Failed to update cache: {:?}", e);
                }
            }
        });
    }
    
    async fn insert_event(pool: &Pool<Sqlite>, event: &UsageEvent) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO usage_events 
            (timestamp, api_key, function_name, request_id, execution_time_ms, 
             memory_mb, cold_start, success, invocation_cost, compute_cost, total_cost)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.timestamp)
        .bind(&event.api_key)
        .bind(&event.function_name)
        .bind(&event.request_id)
        .bind(event.execution_time_ms)
        .bind(event.memory_mb)
        .bind(event.cold_start)
        .bind(event.success)
        .bind(event.invocation_cost)
        .bind(event.compute_cost)
        .bind(event.total_cost)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    /// Get usage statistics for an API key (current month)
    pub async fn get_stats(&self, api_key: &str) -> Result<UsageStats, sqlx::Error> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(api_key) {
                let now = chrono::Utc::now().timestamp();
                if now - cached.last_updated < 60 {
                    // Cache valid for 60 seconds
                    return Ok(cached.stats.clone());
                }
            }
        }
        
        // Calculate from database
        let stats = self.calculate_current_month_stats(api_key).await?;
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                api_key.to_string(),
                CachedStats {
                    stats: stats.clone(),
                    last_updated: chrono::Utc::now().timestamp(),
                },
            );
        }
        
        Ok(stats)
    }
    
    async fn calculate_current_month_stats(&self, api_key: &str) -> Result<UsageStats, sqlx::Error> {
        let now = chrono::Utc::now();
        let month_start = now.format("%Y-%m-01").to_string();
        let month_end = now.format("%Y-%m-%d").to_string();
        
        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(total_invocations), 0) as total_invocations,
                COALESCE(SUM(successful_invocations), 0) as successful_invocations,
                COALESCE(SUM(failed_invocations), 0) as failed_invocations,
                COALESCE(SUM(total_execution_time_ms), 0) as total_execution_time_ms,
                COALESCE(SUM(total_memory_mb_seconds), 0.0) as total_memory_mb_seconds,
                COALESCE(SUM(cold_starts), 0) as cold_starts,
                COALESCE(SUM(total_cost), 0.0) as total_cost
            FROM usage_stats
            WHERE api_key = ? AND date >= ? AND date <= ?
            "#,
        )
        .bind(api_key)
        .bind(&month_start)
        .bind(&month_end)
        .fetch_one(&self.pool)
        .await?;
        
        // Get unique functions used
        let functions_row = sqlx::query(
            r#"
            SELECT DISTINCT function_name
            FROM usage_events
            WHERE api_key = ? AND timestamp >= ?
            ORDER BY function_name
            "#,
        )
        .bind(api_key)
        .bind(chrono::NaiveDate::parse_from_str(&month_start, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp())
        .fetch_all(&self.pool)
        .await?;
        
        let functions_used: Vec<String> = functions_row
            .iter()
            .map(|r| r.get::<String, _>("function_name"))
            .collect();
        
        Ok(UsageStats {
            api_key: api_key.to_string(),
            date: month_end,
            total_invocations: row.get::<i64, _>("total_invocations") as u64,
            successful_invocations: row.get::<i64, _>("successful_invocations") as u64,
            failed_invocations: row.get::<i64, _>("failed_invocations") as u64,
            total_execution_time_ms: row.get::<i64, _>("total_execution_time_ms") as u64,
            total_memory_mb_seconds: row.get::<f64, _>("total_memory_mb_seconds"),
            cold_starts: row.get::<i64, _>("cold_starts") as u64,
            total_cost: row.get::<f64, _>("total_cost"),
            functions_used: serde_json::to_string(&functions_used).unwrap_or_else(|_| "[]".to_string()),
        })
    }
    
    async fn update_cache(
        pool: &Pool<Sqlite>,
        cache: &Arc<RwLock<HashMap<String, CachedStats>>>,
        api_key: &str,
    ) -> Result<(), sqlx::Error> {
        // This would be called after inserting an event
        // For now, we'll just invalidate the cache
        let mut cache = cache.write().await;
        cache.remove(api_key);
        Ok(())
    }
    
    /// Get billing information for a time period
    pub async fn get_billing_info(
        &self,
        api_key: &str,
        period_start: i64,
        period_end: i64,
    ) -> Result<BillingInfo, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                date,
                total_invocations,
                total_cost
            FROM usage_stats
            WHERE api_key = ? 
              AND date >= ? 
              AND date <= ?
            ORDER BY date
            "#,
        )
        .bind(api_key)
        .bind(chrono::DateTime::from_timestamp(period_start, 0)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string())
        .bind(chrono::DateTime::from_timestamp(period_end, 0)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string())
        .fetch_all(&self.pool)
        .await?;
        
        let mut total_invocations = 0u64;
        let mut total_cost = 0.0;
        let mut breakdown = Vec::new();
        
        for row in rows {
            let invocations = row.get::<i64, _>("total_invocations") as u64;
            let cost = row.get::<f64, _>("total_cost");
            
            total_invocations += invocations;
            total_cost += cost;
            
            breakdown.push(DailyCost {
                date: row.get("date"),
                invocations,
                cost,
            });
        }
        
        // Calculate invocation vs compute cost split (approximate)
        let invocation_cost = total_invocations as f64 * 0.00000016;
        let compute_cost = total_cost - invocation_cost;
        
        Ok(BillingInfo {
            api_key: api_key.to_string(),
            period_start,
            period_end,
            total_invocations,
            total_cost,
            invocation_cost,
            compute_cost,
            breakdown,
        })
    }
    
    /// Run daily aggregation job (should be run once per day)
    pub async fn aggregate_daily_stats(&self) -> Result<(), sqlx::Error> {
        let yesterday = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(1))
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        
        let yesterday_start = chrono::NaiveDate::parse_from_str(&yesterday, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        
        let yesterday_end = yesterday_start + 86400;
        
        // Get all API keys that had activity yesterday
        let api_keys: Vec<String> = sqlx::query(
            "SELECT DISTINCT api_key FROM usage_events 
             WHERE timestamp >= ? AND timestamp < ?",
        )
        .bind(yesterday_start)
        .bind(yesterday_end)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|r| r.get("api_key"))
        .collect();
        
        for api_key in api_keys {
            self.aggregate_for_api_key(&api_key, &yesterday, yesterday_start, yesterday_end)
                .await?;
        }
        
        tracing::info!("Daily aggregation completed for {}", yesterday);
        Ok(())
    }
    
    async fn aggregate_for_api_key(
        &self,
        api_key: &str,
        date: &str,
        start: i64,
        end: i64,
    ) -> Result<(), sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_invocations,
                SUM(CASE WHEN success THEN 1 ELSE 0 END) as successful_invocations,
                SUM(CASE WHEN NOT success THEN 1 ELSE 0 END) as failed_invocations,
                SUM(execution_time_ms) as total_execution_time_ms,
                SUM(CAST(memory_mb as REAL) * execution_time_ms / 1000.0) as total_memory_mb_seconds,
                SUM(CASE WHEN cold_start THEN 1 ELSE 0 END) as cold_starts,
                SUM(total_cost) as total_cost
            FROM usage_events
            WHERE api_key = ? AND timestamp >= ? AND timestamp < ?
            "#,
        )
        .bind(api_key)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;
        
        // Get unique functions
        let functions: Vec<String> = sqlx::query(
            "SELECT DISTINCT function_name FROM usage_events 
             WHERE api_key = ? AND timestamp >= ? AND timestamp < ?",
        )
        .bind(api_key)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|r| r.get("function_name"))
        .collect();
        
        // Insert or update aggregated stats
        sqlx::query(
            r#"
            INSERT INTO usage_stats 
            (api_key, date, total_invocations, successful_invocations, failed_invocations,
             total_execution_time_ms, total_memory_mb_seconds, cold_starts, total_cost, functions_used)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(api_key, date) DO UPDATE SET
                total_invocations = excluded.total_invocations,
                successful_invocations = excluded.successful_invocations,
                failed_invocations = excluded.failed_invocations,
                total_execution_time_ms = excluded.total_execution_time_ms,
                total_memory_mb_seconds = excluded.total_memory_mb_seconds,
                cold_starts = excluded.cold_starts,
                total_cost = excluded.total_cost,
                functions_used = excluded.functions_used
            "#,
        )
        .bind(api_key)
        .bind(date)
        .bind(row.get::<i64, _>("total_invocations"))
        .bind(row.get::<i64, _>("successful_invocations"))
        .bind(row.get::<i64, _>("failed_invocations"))
        .bind(row.get::<i64, _>("total_execution_time_ms"))
        .bind(row.get::<f64, _>("total_memory_mb_seconds"))
        .bind(row.get::<i64, _>("cold_starts"))
        .bind(row.get::<f64, _>("total_cost"))
        .bind(serde_json::to_string(&functions).unwrap_or_else(|_| "[]".to_string()))
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}
