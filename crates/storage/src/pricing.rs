//! Dynamic pricing configuration
//!
//! This module provides a flexible pricing system that can be updated without code changes.
//! Pricing is stored in the database and cached in memory for performance.

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Pricing configuration for the platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Price per invocation (e.g., 0.00000016 = $0.16 per 1M)
    pub price_per_invocation: f64,
    
    /// Price per GB-second (e.g., 0.000015 = $0.015 per GB-second)
    pub price_per_gb_second: f64,
    
    /// Version number for tracking changes
    pub version: i32,
    
    /// Effective date (Unix timestamp)
    pub effective_date: i64,
    
    /// Notes about this pricing change
    pub notes: String,
    
    /// Whether this is the active pricing
    pub is_active: bool,
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            price_per_invocation: 0.00000016, // $0.16 per 1M (20% cheaper than AWS)
            price_per_gb_second: 0.000015,    // $0.000015 per GB-second
            version: 1,
            effective_date: chrono::Utc::now().timestamp(),
            notes: "Initial pricing: 20% cheaper than AWS Lambda".to_string(),
            is_active: true,
        }
    }
}

impl PricingConfig {
    /// Calculate invocation cost
    pub fn calculate_invocation_cost(&self, invocations: u64) -> f64 {
        invocations as f64 * self.price_per_invocation
    }
    
    /// Calculate compute cost from memory and execution time
    pub fn calculate_compute_cost(&self, memory_mb: u32, execution_time_ms: i64) -> f64 {
        let gb_seconds = (memory_mb as f64 / 1024.0) * (execution_time_ms as f64 / 1000.0);
        gb_seconds * self.price_per_gb_second
    }
    
    /// Calculate total cost
    pub fn calculate_total_cost(&self, invocations: u64, memory_mb: u32, execution_time_ms: i64) -> f64 {
        self.calculate_invocation_cost(invocations) + self.calculate_compute_cost(memory_mb, execution_time_ms)
    }
}

/// Pricing manager with database persistence and in-memory caching
pub struct PricingManager {
    pool: SqlitePool,
    cache: Arc<RwLock<PricingConfig>>,
}

impl PricingManager {
    /// Create new pricing manager
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        let manager = Self {
            pool: pool.clone(),
            cache: Arc::new(RwLock::new(PricingConfig::default())),
        };
        
        manager.init_table().await?;
        manager.load_active_pricing().await?;
        
        Ok(manager)
    }
    
    /// Initialize pricing table
    async fn init_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pricing_config (
                version INTEGER PRIMARY KEY,
                price_per_invocation REAL NOT NULL,
                price_per_gb_second REAL NOT NULL,
                effective_date INTEGER NOT NULL,
                notes TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Insert default pricing if table is empty
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pricing_config")
            .fetch_one(&self.pool)
            .await?;
        
        if count == 0 {
            let default = PricingConfig::default();
            self.save_pricing(&default).await?;
        }
        
        Ok(())
    }
    
    /// Load active pricing from database into cache
    async fn load_active_pricing(&self) -> Result<(), sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT version, price_per_invocation, price_per_gb_second, 
                   effective_date, notes, is_active
            FROM pricing_config
            WHERE is_active = 1
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        
        let pricing = PricingConfig {
            version: row.get("version"),
            price_per_invocation: row.get("price_per_invocation"),
            price_per_gb_second: row.get("price_per_gb_second"),
            effective_date: row.get("effective_date"),
            notes: row.get("notes"),
            is_active: row.get("is_active"),
        };
        
        let mut cache = self.cache.write().await;
        *cache = pricing;
        
        Ok(())
    }
    
    /// Get current pricing (from cache)
    pub async fn get_pricing(&self) -> PricingConfig {
        self.cache.read().await.clone()
    }
    
    /// Save new pricing version to database
    pub async fn save_pricing(&self, pricing: &PricingConfig) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO pricing_config 
            (version, price_per_invocation, price_per_gb_second, effective_date, notes, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(pricing.version)
        .bind(pricing.price_per_invocation)
        .bind(pricing.price_per_gb_second)
        .bind(pricing.effective_date)
        .bind(&pricing.notes)
        .bind(pricing.is_active)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Update pricing (creates new version, deactivates old)
    pub async fn update_pricing(
        &self,
        price_per_invocation: f64,
        price_per_gb_second: f64,
        notes: String,
    ) -> Result<PricingConfig, sqlx::Error> {
        // Get current version
        let current_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) FROM pricing_config"
        )
        .fetch_one(&self.pool)
        .await?;
        
        // Deactivate all previous pricing
        sqlx::query("UPDATE pricing_config SET is_active = 0")
            .execute(&self.pool)
            .await?;
        
        // Create new pricing version
        let new_pricing = PricingConfig {
            version: current_version + 1,
            price_per_invocation,
            price_per_gb_second,
            effective_date: chrono::Utc::now().timestamp(),
            notes,
            is_active: true,
        };
        
        self.save_pricing(&new_pricing).await?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        *cache = new_pricing.clone();
        
        tracing::info!(
            "Pricing updated to v{}: ${}/1M invocations, ${}/GB-second",
            new_pricing.version,
            new_pricing.price_per_invocation * 1_000_000.0,
            new_pricing.price_per_gb_second
        );
        
        Ok(new_pricing)
    }
    
    /// Get pricing history
    pub async fn get_pricing_history(&self) -> Result<Vec<PricingConfig>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT version, price_per_invocation, price_per_gb_second,
                   effective_date, notes, is_active
            FROM pricing_config
            ORDER BY version DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        
        let history = rows
            .iter()
            .map(|row| PricingConfig {
                version: row.get("version"),
                price_per_invocation: row.get("price_per_invocation"),
                price_per_gb_second: row.get("price_per_gb_second"),
                effective_date: row.get("effective_date"),
                notes: row.get("notes"),
                is_active: row.get("is_active"),
            })
            .collect();
        
        Ok(history)
    }
    
    /// Get pricing at specific timestamp (for historical billing)
    pub async fn get_pricing_at_timestamp(&self, timestamp: i64) -> Result<PricingConfig, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT version, price_per_invocation, price_per_gb_second,
                   effective_date, notes, is_active
            FROM pricing_config
            WHERE effective_date <= ?
            ORDER BY effective_date DESC
            LIMIT 1
            "#,
        )
        .bind(timestamp)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(PricingConfig {
            version: row.get("version"),
            price_per_invocation: row.get("price_per_invocation"),
            price_per_gb_second: row.get("price_per_gb_second"),
            effective_date: row.get("effective_date"),
            notes: row.get("notes"),
            is_active: row.get("is_active"),
        })
    }
    
    /// Reload pricing from database (useful after external updates)
    pub async fn reload(&self) -> Result<(), sqlx::Error> {
        self.load_active_pricing().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pricing_manager() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let manager = PricingManager::new(pool).await.unwrap();
        
        // Get default pricing
        let pricing = manager.get_pricing().await;
        assert_eq!(pricing.version, 1);
        assert_eq!(pricing.price_per_invocation, 0.00000016);
        
        // Update pricing
        let new_pricing = manager
            .update_pricing(0.0000002, 0.00002, "Price increase test".to_string())
            .await
            .unwrap();
        
        assert_eq!(new_pricing.version, 2);
        assert_eq!(new_pricing.price_per_invocation, 0.0000002);
        
        // Verify cache updated
        let cached = manager.get_pricing().await;
        assert_eq!(cached.version, 2);
        
        // Check history
        let history = manager.get_pricing_history().await.unwrap();
        assert_eq!(history.len(), 2);
    }
}
