use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

const TRIAL_DURATION_DAYS: i64 = 14;
const TRIAL_INVOCATION_LIMIT: i64 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialStatus {
    pub api_key: String,
    pub trial_start: i64,
    pub trial_end: i64,
    pub invocations_used: i64,
    pub trial_active: bool,
    pub days_remaining: i64,
    pub invocations_remaining: i64,
}

impl TrialStatus {
    /// Check if trial is still valid
    pub fn is_valid(&self) -> bool {
        if !self.trial_active {
            return false;
        }

        let now = chrono::Utc::now().timestamp();
        let time_valid = now < self.trial_end;
        let invocations_valid = self.invocations_used < TRIAL_INVOCATION_LIMIT;

        time_valid && invocations_valid
    }

    /// Get reason for trial expiration
    pub fn expiration_reason(&self) -> Option<String> {
        if self.is_valid() {
            return None;
        }

        let now = chrono::Utc::now().timestamp();
        if now >= self.trial_end {
            Some("Trial period expired (14 days elapsed)".to_string())
        } else if self.invocations_used >= TRIAL_INVOCATION_LIMIT {
            Some(format!(
                "Trial invocation limit reached ({} invocations used)",
                self.invocations_used
            ))
        } else {
            Some("Trial inactive".to_string())
        }
    }
}

pub struct TrialManager {
    pool: SqlitePool,
}

impl TrialManager {
    pub async fn new(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        let manager = Self { pool };
        manager.init_db().await?;
        Ok(manager)
    }

    async fn init_db(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trial_status (
                api_key TEXT PRIMARY KEY,
                trial_start INTEGER NOT NULL,
                trial_end INTEGER NOT NULL,
                invocations_used INTEGER NOT NULL DEFAULT 0,
                trial_active BOOLEAN NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for quick lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_trial_api_key 
            ON trial_status(api_key)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Start a trial for a new API key
    pub async fn start_trial(&self, api_key: &str) -> Result<TrialStatus, sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        let trial_end = now + (TRIAL_DURATION_DAYS * 24 * 60 * 60);

        sqlx::query(
            r#"
            INSERT INTO trial_status (api_key, trial_start, trial_end, invocations_used, trial_active, created_at)
            VALUES (?, ?, ?, 0, 1, ?)
            "#
        )
        .bind(api_key)
        .bind(now)
        .bind(trial_end)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_trial_status(api_key).await
    }

    /// Get trial status for an API key
    pub async fn get_trial_status(&self, api_key: &str) -> Result<TrialStatus, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT api_key, trial_start, trial_end, invocations_used, trial_active
            FROM trial_status
            WHERE api_key = ?
            "#,
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await?;

        let trial_start: i64 = row.get("trial_start");
        let trial_end: i64 = row.get("trial_end");
        let invocations_used: i64 = row.get("invocations_used");
        let trial_active: bool = row.get("trial_active");

        let now = chrono::Utc::now().timestamp();
        let days_remaining = ((trial_end - now) / (24 * 60 * 60)).max(0);
        let invocations_remaining = (TRIAL_INVOCATION_LIMIT - invocations_used).max(0);

        Ok(TrialStatus {
            api_key: api_key.to_string(),
            trial_start,
            trial_end,
            invocations_used,
            trial_active,
            days_remaining,
            invocations_remaining,
        })
    }

    /// Check if trial exists for an API key
    pub async fn has_trial(&self, api_key: &str) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM trial_status WHERE api_key = ?
            "#,
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Increment invocation counter for trial
    pub async fn increment_invocation(&self, api_key: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE trial_status 
            SET invocations_used = invocations_used + 1
            WHERE api_key = ?
            "#,
        )
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deactivate trial (when user upgrades to paid)
    pub async fn deactivate_trial(&self, api_key: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE trial_status 
            SET trial_active = 0
            WHERE api_key = ?
            "#,
        )
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all active trials (for admin)
    pub async fn get_all_active_trials(&self) -> Result<Vec<TrialStatus>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT api_key, trial_start, trial_end, invocations_used, trial_active
            FROM trial_status
            WHERE trial_active = 1
            ORDER BY trial_start DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let now = chrono::Utc::now().timestamp();

        Ok(rows
            .into_iter()
            .map(|row| {
                let trial_start: i64 = row.get("trial_start");
                let trial_end: i64 = row.get("trial_end");
                let invocations_used: i64 = row.get("invocations_used");
                let trial_active: bool = row.get("trial_active");
                let api_key: String = row.get("api_key");

                let days_remaining = ((trial_end - now) / (24 * 60 * 60)).max(0);
                let invocations_remaining = (TRIAL_INVOCATION_LIMIT - invocations_used).max(0);

                TrialStatus {
                    api_key,
                    trial_start,
                    trial_end,
                    invocations_used,
                    trial_active,
                    days_remaining,
                    invocations_remaining,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trial_creation() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = TrialManager::new(pool).await.unwrap();

        let trial = manager.start_trial("test_key_1").await.unwrap();

        assert_eq!(trial.api_key, "test_key_1");
        assert_eq!(trial.invocations_used, 0);
        assert_eq!(trial.trial_active, true);
        assert!(trial.is_valid());
        assert_eq!(trial.days_remaining, 14);
        assert_eq!(trial.invocations_remaining, 100_000);
    }

    #[tokio::test]
    async fn test_trial_invocation_increment() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = TrialManager::new(pool).await.unwrap();

        manager.start_trial("test_key_2").await.unwrap();

        for _ in 0..5 {
            manager.increment_invocation("test_key_2").await.unwrap();
        }

        let trial = manager.get_trial_status("test_key_2").await.unwrap();
        assert_eq!(trial.invocations_used, 5);
        assert_eq!(trial.invocations_remaining, 99_995);
    }

    #[tokio::test]
    async fn test_trial_deactivation() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let manager = TrialManager::new(pool).await.unwrap();

        manager.start_trial("test_key_3").await.unwrap();
        manager.deactivate_trial("test_key_3").await.unwrap();

        let trial = manager.get_trial_status("test_key_3").await.unwrap();
        assert_eq!(trial.trial_active, false);
        assert!(!trial.is_valid());
    }
}
