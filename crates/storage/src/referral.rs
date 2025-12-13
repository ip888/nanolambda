// Referral program and tracking module
use crate::error::{Result, StorageError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralCode {
    pub id: i64,
    pub referrer_api_key: String,
    pub code: String,                    // Unique referral code (e.g., "user-abc123xyz")
    pub display_name: String,            // User's display name for sharing
    pub reward_type: String,             // "percentage" or "fixed"
    pub reward_amount: i64,              // Percentage (1-100) or cents
    pub reward_description: String,      // "20% off" or "$10 credit"
    pub max_referrals: Option<i64>,      // Limit on successful referrals
    pub current_referrals: i64,          // Count of successful signups
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralReward {
    pub id: i64,
    pub referral_code_id: i64,
    pub referrer_api_key: String,        // The person who referred
    pub referred_api_key: Option<String>,// The person who signed up (if they became customer)
    pub referred_email: String,          // Email of referred person
    pub status: String,                  // "pending", "activated", "claimed"
    pub reward_earned: bool,             // true if reward was earned
    pub reward_amount: i64,              // Reward in cents
    pub discount_code_id: Option<i64>,   // Discount code generated for them
    pub tracking_data: String,           // JSON with utm params, source, etc
    pub referred_at: i64,                // When they clicked the referral link
    pub activated_at: Option<i64>,       // When they signed up/activated
    pub claimed_at: Option<i64>,         // When reward was applied
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReferralStats {
    pub total_referrals: i64,            // Total referral signups
    pub active_referrals: i64,           // Successful referrals with reward earned
    pub pending_referrals: i64,          // Pending verification
    pub total_rewards_earned: i64,       // Total rewards in cents
    pub total_rewards_value: String,     // Formatted as "$X.XX"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralLeaderboard {
    pub referrer_api_key: String,
    pub display_name: String,
    pub referral_code: String,
    pub successful_referrals: i64,
    pub total_rewards_earned: i64,
    pub rank: i64,
}

pub struct ReferralManager {
    pool: SqlitePool,
}

impl ReferralManager {
    /// Create a new ReferralManager
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        // Create referral_codes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS referral_codes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                referrer_api_key TEXT NOT NULL UNIQUE,
                code TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                reward_type TEXT NOT NULL,
                reward_amount INTEGER NOT NULL,
                reward_description TEXT NOT NULL,
                max_referrals INTEGER,
                current_referrals INTEGER NOT NULL DEFAULT 0,
                active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create referral_rewards table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS referral_rewards (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                referral_code_id INTEGER NOT NULL,
                referrer_api_key TEXT NOT NULL,
                referred_api_key TEXT,
                referred_email TEXT NOT NULL,
                status TEXT NOT NULL,
                reward_earned INTEGER NOT NULL DEFAULT 0,
                reward_amount INTEGER NOT NULL,
                discount_code_id INTEGER,
                tracking_data TEXT NOT NULL,
                referred_at INTEGER NOT NULL,
                activated_at INTEGER,
                claimed_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (referral_code_id) REFERENCES referral_codes(id) ON DELETE CASCADE,
                FOREIGN KEY (discount_code_id) REFERENCES discount_codes(id) ON DELETE SET NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create indices for performance
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_referral_codes_referrer 
            ON referral_codes(referrer_api_key)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_referral_codes_code 
            ON referral_codes(code)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_referral_rewards_referrer 
            ON referral_rewards(referrer_api_key)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_referral_rewards_code_id 
            ON referral_rewards(referral_code_id)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_referral_rewards_status 
            ON referral_rewards(status)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Generate a referral code for a user (called on first signup)
    pub async fn generate_referral_code(
        &self,
        referrer_api_key: &str,
        display_name: &str,
        reward_type: &str,
        reward_amount: i64,
        reward_description: &str,
        max_referrals: Option<i64>,
    ) -> Result<ReferralCode> {
        // Generate unique code from API key hash
        let code = format!(
            "ref-{}-{}",
            display_name
                .to_lowercase()
                .replace(" ", "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .take(20)
                .collect::<String>(),
            Uuid::new_v4().to_string()[..8].to_string()
        );

        let now = Utc::now().timestamp();

        let id = sqlx::query(
            r#"
            INSERT INTO referral_codes (
                referrer_api_key, code, display_name, reward_type, 
                reward_amount, reward_description, max_referrals, 
                current_referrals, active, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?)
            "#,
        )
        .bind(referrer_api_key)
        .bind(&code)
        .bind(display_name)
        .bind(reward_type)
        .bind(reward_amount)
        .bind(reward_description)
        .bind(max_referrals)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .last_insert_rowid();

        Ok(ReferralCode {
            id,
            referrer_api_key: referrer_api_key.to_string(),
            code,
            display_name: display_name.to_string(),
            reward_type: reward_type.to_string(),
            reward_amount,
            reward_description: reward_description.to_string(),
            max_referrals,
            current_referrals: 0,
            active: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get referral code for a user
    pub async fn get_referral_code(&self, api_key: &str) -> Result<Option<ReferralCode>> {
        let row = sqlx::query(
            r#"
            SELECT id, referrer_api_key, code, display_name, reward_type, reward_amount,
                   reward_description, max_referrals, current_referrals, active, created_at, updated_at
            FROM referral_codes
            WHERE referrer_api_key = ?
            "#,
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| ReferralCode {
            id: r.get("id"),
            referrer_api_key: r.get("referrer_api_key"),
            code: r.get("code"),
            display_name: r.get("display_name"),
            reward_type: r.get("reward_type"),
            reward_amount: r.get("reward_amount"),
            reward_description: r.get("reward_description"),
            max_referrals: r.get("max_referrals"),
            current_referrals: r.get("current_referrals"),
            active: r.get::<i64, _>("active") == 1,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Get referral code by code string
    pub async fn get_referral_code_by_code(&self, code: &str) -> Result<Option<ReferralCode>> {
        let row = sqlx::query(
            r#"
            SELECT id, referrer_api_key, code, display_name, reward_type, reward_amount,
                   reward_description, max_referrals, current_referrals, active, created_at, updated_at
            FROM referral_codes
            WHERE code = ?
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| ReferralCode {
            id: r.get("id"),
            referrer_api_key: r.get("referrer_api_key"),
            code: r.get("code"),
            display_name: r.get("display_name"),
            reward_type: r.get("reward_type"),
            reward_amount: r.get("reward_amount"),
            reward_description: r.get("reward_description"),
            max_referrals: r.get("max_referrals"),
            current_referrals: r.get("current_referrals"),
            active: r.get::<i64, _>("active") == 1,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Track a referral click (when someone uses a referral link)
    pub async fn track_referral_click(
        &self,
        referral_code_id: i64,
        referred_email: &str,
        tracking_data: &str, // JSON with utm_source, utm_campaign, etc
    ) -> Result<ReferralReward> {
        // Get referral code details
        let code_row = sqlx::query(
            r#"
            SELECT referrer_api_key, reward_amount
            FROM referral_codes
            WHERE id = ?
            "#,
        )
        .bind(referral_code_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .ok_or_else(|| StorageError::NotFound)?;

        let referrer_api_key: String = code_row.get("referrer_api_key");
        let reward_amount: i64 = code_row.get("reward_amount");
        let now = Utc::now().timestamp();

        let id = sqlx::query(
            r#"
            INSERT INTO referral_rewards (
                referral_code_id, referrer_api_key, referred_email,
                status, reward_earned, reward_amount, tracking_data,
                referred_at, created_at, updated_at
            )
            VALUES (?, ?, ?, 'pending', 0, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(referral_code_id)
        .bind(&referrer_api_key)
        .bind(referred_email)
        .bind(reward_amount)
        .bind(tracking_data)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .last_insert_rowid();

        Ok(ReferralReward {
            id,
            referral_code_id,
            referrer_api_key,
            referred_api_key: None,
            referred_email: referred_email.to_string(),
            status: "pending".to_string(),
            reward_earned: false,
            reward_amount,
            discount_code_id: None,
            tracking_data: tracking_data.to_string(),
            referred_at: now,
            activated_at: None,
            claimed_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Mark a referral as activated (when they sign up and become a customer)
    pub async fn activate_referral(
        &self,
        referral_reward_id: i64,
        referred_api_key: &str,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        // Update reward status and referred API key
        sqlx::query(
            r#"
            UPDATE referral_rewards
            SET status = 'activated', referred_api_key = ?, activated_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(referred_api_key)
        .bind(now)
        .bind(now)
        .bind(referral_reward_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Get referral code ID to increment counter
        let reward = sqlx::query("SELECT referral_code_id FROM referral_rewards WHERE id = ?")
            .bind(referral_reward_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let referral_code_id: i64 = reward.get("referral_code_id");

        // Increment current_referrals counter
        sqlx::query(
            r#"
            UPDATE referral_codes
            SET current_referrals = current_referrals + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(referral_code_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Claim a referral reward (apply discount to referrer's account)
    pub async fn claim_referral_reward(
        &self,
        referral_reward_id: i64,
        discount_code_id: i64,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE referral_rewards
            SET status = 'claimed', reward_earned = 1, discount_code_id = ?, claimed_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(discount_code_id)
        .bind(now)
        .bind(now)
        .bind(referral_reward_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get stats for a referrer
    pub async fn get_referral_stats(&self, api_key: &str) -> Result<ReferralStats> {
        let stats_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_referrals,
                SUM(CASE WHEN status = 'activated' THEN 1 ELSE 0 END) as active_referrals,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending_referrals,
                SUM(CASE WHEN reward_earned = 1 THEN reward_amount ELSE 0 END) as total_rewards
            FROM referral_rewards
            WHERE referrer_api_key = ?
            "#,
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let total_referrals: i64 = stats_row.get::<Option<i64>, _>("total_referrals").unwrap_or(0);
        let active_referrals: i64 = stats_row.get::<Option<i64>, _>("active_referrals").unwrap_or(0);
        let pending_referrals: i64 = stats_row.get::<Option<i64>, _>("pending_referrals").unwrap_or(0);
        let total_rewards_earned: i64 = stats_row.get::<Option<i64>, _>("total_rewards").unwrap_or(0);

        Ok(ReferralStats {
            total_referrals,
            active_referrals,
            pending_referrals,
            total_rewards_earned,
            total_rewards_value: format!("${:.2}", total_rewards_earned as f64 / 100.0),
        })
    }

    /// Get all rewards for a referrer
    pub async fn get_referral_rewards(&self, api_key: &str) -> Result<Vec<ReferralReward>> {
        let rows = sqlx::query(
            r#"
            SELECT id, referral_code_id, referrer_api_key, referred_api_key, referred_email,
                   status, reward_earned, reward_amount, discount_code_id, tracking_data,
                   referred_at, activated_at, claimed_at, created_at, updated_at
            FROM referral_rewards
            WHERE referrer_api_key = ?
            ORDER BY referred_at DESC
            "#,
        )
        .bind(api_key)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ReferralReward {
                id: r.get("id"),
                referral_code_id: r.get("referral_code_id"),
                referrer_api_key: r.get("referrer_api_key"),
                referred_api_key: r.get("referred_api_key"),
                referred_email: r.get("referred_email"),
                status: r.get("status"),
                reward_earned: r.get::<i64, _>("reward_earned") == 1,
                reward_amount: r.get("reward_amount"),
                discount_code_id: r.get("discount_code_id"),
                tracking_data: r.get("tracking_data"),
                referred_at: r.get("referred_at"),
                activated_at: r.get("activated_at"),
                claimed_at: r.get("claimed_at"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Get leaderboard of top referrers
    pub async fn get_leaderboard(&self, limit: i64) -> Result<Vec<ReferralLeaderboard>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                rc.referrer_api_key,
                rc.display_name,
                rc.code,
                COUNT(CASE WHEN rr.status = 'activated' THEN 1 END) as successful_referrals,
                SUM(CASE WHEN rr.reward_earned = 1 THEN rr.reward_amount ELSE 0 END) as total_rewards,
                ROW_NUMBER() OVER (ORDER BY COUNT(CASE WHEN rr.status = 'activated' THEN 1 END) DESC) as rank
            FROM referral_codes rc
            LEFT JOIN referral_rewards rr ON rc.id = rr.referral_code_id
            WHERE rc.active = 1
            GROUP BY rc.id
            ORDER BY successful_referrals DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ReferralLeaderboard {
                referrer_api_key: r.get("referrer_api_key"),
                display_name: r.get("display_name"),
                referral_code: r.get("code"),
                successful_referrals: r.get::<Option<i64>, _>("successful_referrals").unwrap_or(0),
                total_rewards_earned: r.get::<Option<i64>, _>("total_rewards").unwrap_or(0),
                rank: r.get("rank"),
            })
            .collect())
    }

    /// Get details of a referral by code (for shared links)
    pub async fn get_referral_details(&self, code: &str) -> Result<Option<(ReferralCode, ReferralStats)>> {
        match self.get_referral_code_by_code(code).await? {
            Some(referral) => {
                let stats = self.get_referral_stats(&referral.referrer_api_key).await?;
                Ok(Some((referral, stats)))
            }
            None => Ok(None),
        }
    }

    /// Check if referral code can accept more referrals
    pub async fn can_accept_referral(&self, referral_code_id: i64) -> Result<bool> {
        let row = sqlx::query(
            r#"
            SELECT max_referrals, current_referrals, active
            FROM referral_codes
            WHERE id = ?
            "#,
        )
        .bind(referral_code_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .ok_or_else(|| StorageError::NotFound)?;

        let active: i64 = row.get("active");
        if active == 0 {
            return Ok(false);
        }

        let max_referrals: Option<i64> = row.get("max_referrals");
        let current_referrals: i64 = row.get("current_referrals");

        Ok(match max_referrals {
            Some(max) => current_referrals < max,
            None => true, // Unlimited
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_referral_code() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let mgr = ReferralManager::new(pool).await.unwrap();

        let code = mgr
            .generate_referral_code(
                "user-123",
                "John Doe",
                "percentage",
                20,
                "20% off first month",
                None,
            )
            .await
            .unwrap();

        assert_eq!(code.referrer_api_key, "user-123");
        assert_eq!(code.reward_amount, 20);
        assert_eq!(code.current_referrals, 0);
    }

    #[tokio::test]
    async fn test_get_referral_code() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let mgr = ReferralManager::new(pool).await.unwrap();

        let code = mgr
            .generate_referral_code(
                "user-123",
                "John Doe",
                "percentage",
                20,
                "20% off first month",
                None,
            )
            .await
            .unwrap();

        let retrieved = mgr.get_referral_code("user-123").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().code, code.code);
    }

    #[tokio::test]
    async fn test_max_referrals_limit() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let mgr = ReferralManager::new(pool).await.unwrap();

        let code = mgr
            .generate_referral_code(
                "user-123",
                "John Doe",
                "percentage",
                20,
                "20% off first month",
                Some(2), // Max 2 referrals
            )
            .await
            .unwrap();

        assert!(mgr.can_accept_referral(code.id).await.unwrap());

        // Track two referrals
        mgr.track_referral_click(code.id, "user1@example.com", "{}")
            .await
            .unwrap();
        mgr.track_referral_click(code.id, "user2@example.com", "{}")
            .await
            .unwrap();

        // Activate both
        let rewards = mgr.get_referral_rewards("user-123").await.unwrap();
        mgr.activate_referral(rewards[0].id, "user-1").await.unwrap();
        mgr.activate_referral(rewards[1].id, "user-2").await.unwrap();

        // Should now be at limit
        assert!(!mgr.can_accept_referral(code.id).await.unwrap());
    }
}
