use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Pricing tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TierLevel {
    Starter,
    Pro,
    Enterprise,
}

impl TierLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TierLevel::Starter => "starter",
            TierLevel::Pro => "pro",
            TierLevel::Enterprise => "enterprise",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "starter" => Some(TierLevel::Starter),
            "pro" => Some(TierLevel::Pro),
            "enterprise" => Some(TierLevel::Enterprise),
            _ => None,
        }
    }
}

/// Tier configuration with pricing and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub tier: TierLevel,
    pub name: String,
    pub description: String,
    
    // Pricing (per invocation and per GB-second)
    pub price_per_invocation: f64,
    pub price_per_gb_second: f64,
    
    // Limits
    pub max_invocations_per_month: Option<i64>, // None = unlimited
    pub max_memory_mb: u32,
    pub max_timeout_ms: i64,
    pub max_concurrent_executions: u32,
    
    // Features
    pub support_level: String,
    pub custom_domains: bool,
    pub advanced_monitoring: bool,
    pub priority_execution: bool,
}

impl TierConfig {
    /// Starter tier: After trial, entry-level paid plan
    pub fn starter() -> Self {
        Self {
            tier: TierLevel::Starter,
            name: "Starter".to_string(),
            description: "Perfect for small projects and development".to_string(),
            price_per_invocation: 0.00000020, // $0.20 per 1M (AWS pricing)
            price_per_gb_second: 0.0000166667, // AWS Lambda pricing
            max_invocations_per_month: Some(1_000_000), // 1M invocations/month
            max_memory_mb: 512,
            max_timeout_ms: 30_000, // 30 seconds
            max_concurrent_executions: 10,
            support_level: "Community (email only)".to_string(),
            custom_domains: false,
            advanced_monitoring: false,
            priority_execution: false,
        }
    }
    
    /// Pro tier: For production applications
    pub fn pro() -> Self {
        Self {
            tier: TierLevel::Pro,
            name: "Pro".to_string(),
            description: "For production workloads with better performance".to_string(),
            price_per_invocation: 0.00000016, // $0.16 per 1M (20% cheaper than AWS)
            price_per_gb_second: 0.000015, // 10% cheaper than AWS
            max_invocations_per_month: Some(10_000_000), // 10M invocations/month
            max_memory_mb: 3072, // 3GB
            max_timeout_ms: 300_000, // 5 minutes
            max_concurrent_executions: 100,
            support_level: "Standard (email + chat, 24h response)".to_string(),
            custom_domains: true,
            advanced_monitoring: true,
            priority_execution: true,
        }
    }
    
    /// Enterprise tier: For large-scale deployments
    pub fn enterprise() -> Self {
        Self {
            tier: TierLevel::Enterprise,
            name: "Enterprise".to_string(),
            description: "Unlimited scale with premium support".to_string(),
            price_per_invocation: 0.00000012, // $0.12 per 1M (40% cheaper than AWS)
            price_per_gb_second: 0.000012, // 28% cheaper than AWS
            max_invocations_per_month: None, // Unlimited
            max_memory_mb: 10240, // 10GB
            max_timeout_ms: 900_000, // 15 minutes
            max_concurrent_executions: 1000,
            support_level: "Premium (24/7 phone + dedicated account manager)".to_string(),
            custom_domains: true,
            advanced_monitoring: true,
            priority_execution: true,
        }
    }
    
    /// Calculate invocation cost
    pub fn calculate_invocation_cost(&self, count: u64) -> f64 {
        self.price_per_invocation * count as f64
    }
    
    /// Calculate compute cost (GB-seconds)
    pub fn calculate_compute_cost(&self, memory_mb: u32, execution_time_ms: i64) -> f64 {
        let gb = memory_mb as f64 / 1024.0;
        let seconds = execution_time_ms as f64 / 1000.0;
        let gb_seconds = gb * seconds;
        gb_seconds * self.price_per_gb_second
    }
    
    /// Check if request is within tier limits
    pub fn check_limits(&self, memory_mb: u32, timeout_ms: i64) -> Result<(), String> {
        if memory_mb > self.max_memory_mb {
            return Err(format!(
                "Memory limit exceeded: {} MB requested, {} MB allowed for {} tier",
                memory_mb, self.max_memory_mb, self.name
            ));
        }
        
        if timeout_ms > self.max_timeout_ms {
            return Err(format!(
                "Timeout limit exceeded: {} ms requested, {} ms allowed for {} tier",
                timeout_ms, self.max_timeout_ms, self.name
            ));
        }
        
        Ok(())
    }
}

/// User's tier assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTier {
    pub api_key: String,
    pub tier: TierLevel,
    pub assigned_at: i64,
    pub monthly_invocations: i64,
    pub month_start: i64, // Unix timestamp of current billing month start
}

pub struct TierManager {
    pool: SqlitePool,
    tier_configs: Arc<RwLock<Vec<TierConfig>>>,
}

impl TierManager {
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        let manager = Self {
            pool: pool.clone(),
            tier_configs: Arc::new(RwLock::new(vec![
                TierConfig::starter(),
                TierConfig::pro(),
                TierConfig::enterprise(),
            ])),
        };
        
        manager.init_db().await?;
        Ok(manager)
    }
    
    async fn init_db(&self) -> Result<(), sqlx::Error> {
        // User tier assignments
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_tiers (
                api_key TEXT PRIMARY KEY,
                tier TEXT NOT NULL,
                assigned_at INTEGER NOT NULL,
                monthly_invocations INTEGER NOT NULL DEFAULT 0,
                month_start INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#
        )
        .execute(&self.pool)
        .await?;
        
        // Index for quick lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_user_tier_api_key 
            ON user_tiers(api_key)
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Assign tier to a new user (defaults to Starter after trial)
    pub async fn assign_tier(&self, api_key: &str, tier: TierLevel) -> Result<UserTier, sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query(
            r#"
            INSERT INTO user_tiers (api_key, tier, assigned_at, monthly_invocations, month_start, created_at)
            VALUES (?, ?, ?, 0, ?, ?)
            ON CONFLICT(api_key) DO UPDATE SET
                tier = excluded.tier,
                assigned_at = excluded.assigned_at
            "#
        )
        .bind(api_key)
        .bind(tier.as_str())
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        
        self.get_user_tier(api_key).await
    }
    
    /// Get user's current tier
    pub async fn get_user_tier(&self, api_key: &str) -> Result<UserTier, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT api_key, tier, assigned_at, monthly_invocations, month_start
            FROM user_tiers
            WHERE api_key = ?
            "#
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await?;
        
        let tier_str: String = row.get("tier");
        let tier = TierLevel::from_str(&tier_str).unwrap_or(TierLevel::Starter);
        
        Ok(UserTier {
            api_key: api_key.to_string(),
            tier,
            assigned_at: row.get("assigned_at"),
            monthly_invocations: row.get("monthly_invocations"),
            month_start: row.get("month_start"),
        })
    }
    
    /// Get tier configuration
    pub async fn get_tier_config(&self, tier: TierLevel) -> TierConfig {
        let configs = self.tier_configs.read().await;
        configs.iter()
            .find(|c| c.tier == tier)
            .cloned()
            .unwrap_or_else(|| TierConfig::starter())
    }
    
    /// Get all available tier plans
    pub async fn get_all_tiers(&self) -> Vec<TierConfig> {
        self.tier_configs.read().await.clone()
    }
    
    /// Increment monthly invocation counter
    pub async fn increment_monthly_invocations(&self, api_key: &str) -> Result<(), sqlx::Error> {
        // Check if we need to reset counter (new month)
        let now = chrono::Utc::now().timestamp();
        
        sqlx::query(
            r#"
            UPDATE user_tiers
            SET monthly_invocations = CASE
                    WHEN ? - month_start > 2592000 THEN 1
                    ELSE monthly_invocations + 1
                END,
                month_start = CASE
                    WHEN ? - month_start > 2592000 THEN ?
                    ELSE month_start
                END
            WHERE api_key = ?
            "#
        )
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(api_key)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Check if user has exceeded monthly limits
    pub async fn check_monthly_limit(&self, api_key: &str) -> Result<bool, sqlx::Error> {
        let user_tier = self.get_user_tier(api_key).await?;
        let tier_config = self.get_tier_config(user_tier.tier).await;
        
        if let Some(max) = tier_config.max_invocations_per_month {
            Ok(user_tier.monthly_invocations < max)
        } else {
            Ok(true) // Unlimited
        }
    }
    
    /// Upgrade user to a higher tier
    pub async fn upgrade_tier(&self, api_key: &str, new_tier: TierLevel) -> Result<UserTier, sqlx::Error> {
        self.assign_tier(api_key, new_tier).await
    }
    
    /// Get intelligent upgrade recommendation based on usage patterns
    pub async fn get_upgrade_recommendation(&self, api_key: &str) -> Result<Option<UpgradeRecommendation>, sqlx::Error> {
        let user_tier = self.get_user_tier(api_key).await?;
        let current_config = self.get_tier_config(user_tier.tier).await;
        
        // Don't recommend upgrades for Enterprise tier
        if user_tier.tier == TierLevel::Enterprise {
            return Ok(None);
        }
        
        // Calculate usage percentage
        let usage_percent = if let Some(limit) = current_config.max_invocations_per_month {
            if limit > 0 {
                (user_tier.monthly_invocations as f64 / limit as f64) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Determine next tier
        let recommended_tier = match user_tier.tier {
            TierLevel::Starter => TierLevel::Pro,
            TierLevel::Pro => TierLevel::Enterprise,
            TierLevel::Enterprise => return Ok(None),
        };
        
        let recommended_config = self.get_tier_config(recommended_tier).await;
        
        // Calculate urgency score (0-100)
        let urgency = if usage_percent >= 95.0 {
            100
        } else if usage_percent >= 85.0 {
            80
        } else if usage_percent >= 70.0 {
            60
        } else if usage_percent >= 50.0 {
            40
        } else {
            20
        };
        
        // Only recommend if usage is at least 50% or approaching limits
        if usage_percent < 50.0 && urgency < 60 {
            return Ok(None);
        }
        
        // Build reasons list
        let mut reasons = Vec::new();
        
        if usage_percent >= 70.0 {
            reasons.push(format!(
                "You've used {:.0}% of your monthly invocation limit",
                usage_percent
            ));
        }
        
        if usage_percent >= 85.0 {
            reasons.push("You're at risk of hitting your monthly limit".to_string());
        }
        
        // Compare benefits
        let invocation_increase = if let (Some(current), Some(next)) = (
            current_config.max_invocations_per_month,
            recommended_config.max_invocations_per_month,
        ) {
            ((next - current) as f64 / current as f64 * 100.0) as i32
        } else {
            0
        };
        
        if invocation_increase > 0 {
            reasons.push(format!("Get {}x more invocations", invocation_increase / 100 + 1));
        }
        
        let memory_increase = recommended_config.max_memory_mb as i32 - current_config.max_memory_mb as i32;
        if memory_increase > 0 {
            reasons.push(format!("Unlock {}MB more memory", memory_increase));
        }
        
        if !current_config.advanced_monitoring && recommended_config.advanced_monitoring {
            reasons.push("Access advanced monitoring features".to_string());
        }
        
        if !current_config.custom_domains && recommended_config.custom_domains {
            reasons.push("Enable custom domains".to_string());
        }
        
        Ok(Some(UpgradeRecommendation {
            current_tier: user_tier.tier,
            recommended_tier,
            urgency,
            usage_percent: usage_percent as u32,
            current_usage: user_tier.monthly_invocations,
            current_limit: current_config.max_invocations_per_month.unwrap_or(0),
            reasons,
            estimated_savings: None, // Can be calculated if cost data available
        }))
    }
}

/// Upgrade recommendation with intelligent analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeRecommendation {
    pub current_tier: TierLevel,
    pub recommended_tier: TierLevel,
    pub urgency: u32, // 0-100, higher = more urgent
    pub usage_percent: u32,
    pub current_usage: i64,
    pub current_limit: i64,
    pub reasons: Vec<String>,
    pub estimated_savings: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tier_assignment() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = TierManager::new(pool).await.unwrap();
        
        let user_tier = manager.assign_tier("test_key_1", TierLevel::Pro).await.unwrap();
        
        assert_eq!(user_tier.tier, TierLevel::Pro);
        assert_eq!(user_tier.monthly_invocations, 0);
    }
    
    #[tokio::test]
    async fn test_tier_configs() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = TierManager::new(pool).await.unwrap();
        
        let starter = manager.get_tier_config(TierLevel::Starter).await;
        let pro = manager.get_tier_config(TierLevel::Pro).await;
        let enterprise = manager.get_tier_config(TierLevel::Enterprise).await;
        
        // Pro should be cheaper than Starter
        assert!(pro.price_per_invocation < starter.price_per_invocation);
        
        // Enterprise should be cheapest
        assert!(enterprise.price_per_invocation < pro.price_per_invocation);
        
        // Higher tiers have higher limits
        assert!(pro.max_memory_mb > starter.max_memory_mb);
        assert!(enterprise.max_memory_mb > pro.max_memory_mb);
    }
    
    #[tokio::test]
    async fn test_monthly_counter() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = TierManager::new(pool).await.unwrap();
        
        manager.assign_tier("test_key_2", TierLevel::Starter).await.unwrap();
        
        for _ in 0..5 {
            manager.increment_monthly_invocations("test_key_2").await.unwrap();
        }
        
        let user_tier = manager.get_user_tier("test_key_2").await.unwrap();
        assert_eq!(user_tier.monthly_invocations, 5);
    }
    
    #[tokio::test]
    async fn test_tier_limits() {
        let starter = TierConfig::starter();
        
        // Within limits
        assert!(starter.check_limits(512, 30000).is_ok());
        
        // Exceeds memory limit
        assert!(starter.check_limits(1024, 30000).is_err());
        
        // Exceeds timeout limit
        assert!(starter.check_limits(512, 60000).is_err());
    }
}
