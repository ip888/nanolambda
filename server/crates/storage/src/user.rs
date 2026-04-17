//! User management for NanoLambda
//!
//! This module handles user registration, authentication, profile management,
//! and usage tracking for the NanoLambda serverless platform.
//!
//! ## Features
//!
//! - **User registration**: Email/password registration with validation
//! - **Authentication**: Secure password hashing with Argon2id
//! - **Plan management**: Free, Pro, Scale, and Enterprise tiers
//! - **Usage tracking**: Monthly invocation, CPU, memory, and bandwidth tracking
//! - **Limit enforcement**: Automatic enforcement of plan-based resource limits
//!
//! ## Subscription Plans
//!
//! | Plan | Price | Invocations/Mo | Memory | Functions | Cron Jobs |
//! |------|-------|----------------|--------|-----------|-----------|
//! | Free | $0 | 100K | 512 MB | 5 | 0 |
//! | Pro | $29 | 1M | 1 GB | 50 | 10 |
//! | Scale | $149 | 10M | 2 GB | 200 | 100 |
//! | Enterprise | Custom | Unlimited | 4 GB | Unlimited | Unlimited |
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! // Register a new user
//! let user = user_manager.register(RegisterRequest {
//!     email: "user@example.com".into(),
//!     password: "secure_password".into(),
//!     name: Some("John Doe".into()),
//! }).await?;
//!
//! // Check usage limits before invocation
//! let limits = user_manager.check_limits(&user.id).await?;
//! if limits.within_limits {
//!     // Proceed with function invocation
//! }
//! ```

use anyhow::{Result, anyhow};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

/// User account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: Option<String>,
    pub plan: Plan,
    pub stripe_customer_id: Option<String>,
    pub email_verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

/// Subscription plan levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    #[default]
    Free,
    Pro,
    Scale,
    Enterprise,
}

impl Plan {
    pub fn as_str(&self) -> &str {
        match self {
            Plan::Free => "free",
            Plan::Pro => "pro",
            Plan::Scale => "scale",
            Plan::Enterprise => "enterprise",
        }
    }

    /// Get plan limits
    pub fn limits(&self) -> PlanLimits {
        match self {
            Plan::Free => PlanLimits {
                invocations_per_month: 100_000,
                max_memory_mb: 512,
                max_timeout_seconds: 10,
                max_functions: 5,
                bandwidth_gb: 10,
                vectors_stored: 10_000,
                cron_jobs: 0,
                websocket_connections: 0,
            },
            Plan::Pro => PlanLimits {
                invocations_per_month: 1_000_000,
                max_memory_mb: 1024,
                max_timeout_seconds: 30,
                max_functions: 50,
                bandwidth_gb: 100,
                vectors_stored: 100_000,
                cron_jobs: 10,
                websocket_connections: 100,
            },
            Plan::Scale => PlanLimits {
                invocations_per_month: 10_000_000,
                max_memory_mb: 2048,
                max_timeout_seconds: 60,
                max_functions: 500,
                bandwidth_gb: 1000,
                vectors_stored: 1_000_000,
                cron_jobs: 100,
                websocket_connections: 1000,
            },
            Plan::Enterprise => PlanLimits {
                invocations_per_month: u64::MAX,
                max_memory_mb: 4096,
                max_timeout_seconds: 900,
                max_functions: u32::MAX,
                bandwidth_gb: u64::MAX,
                vectors_stored: u64::MAX,
                cron_jobs: u32::MAX,
                websocket_connections: u32::MAX,
            },
        }
    }

    /// Get monthly price in cents
    pub fn price_cents(&self) -> u64 {
        match self {
            Plan::Free => 0,
            Plan::Pro => 2900,     // $29
            Plan::Scale => 14900,  // $149
            Plan::Enterprise => 0, // Custom pricing
        }
    }
}

impl FromStr for Plan {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "free" => Ok(Plan::Free),
            "pro" => Ok(Plan::Pro),
            "scale" => Ok(Plan::Scale),
            "enterprise" => Ok(Plan::Enterprise),
            _ => Err(anyhow!("Invalid plan: {}", s)),
        }
    }
}

/// Plan limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLimits {
    pub invocations_per_month: u64,
    pub max_memory_mb: u32,
    pub max_timeout_seconds: u32,
    pub max_functions: u32,
    pub bandwidth_gb: u64,
    pub vectors_stored: u64,
    pub cron_jobs: u32,
    pub websocket_connections: u32,
}

/// User registration request
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

/// User login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// User update request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Password change request
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// User usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUsage {
    pub user_id: String,
    pub period_start: i64,
    pub period_end: i64,
    pub invocations: u64,
    pub bandwidth_bytes: u64,
    pub vectors_stored: u64,
    pub functions_count: u32,
    pub cron_jobs_count: u32,
}

/// User manager for database operations
pub struct UserManager {
    pool: SqlitePool,
}

impl UserManager {
    /// Create new user manager
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let manager = Self { pool };
        manager.init_tables().await?;
        Ok(manager)
    }

    /// Initialize database tables
    async fn init_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                name TEXT,
                plan TEXT NOT NULL DEFAULT 'free',
                stripe_customer_id TEXT,
                email_verified INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_login_at INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_users_stripe_customer ON users(stripe_customer_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // User API keys - different from system API keys
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_api_keys (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                name TEXT NOT NULL,
                permissions TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                last_used_at INTEGER,
                status TEXT NOT NULL DEFAULT 'active',
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_user_api_keys_user_id ON user_api_keys(user_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_user_api_keys_hash ON user_api_keys(key_hash)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Sessions table for JWT refresh tokens
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                refresh_token_hash TEXT NOT NULL,
                user_agent TEXT,
                ip_address TEXT,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                revoked_at INTEGER,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Usage tracking per user (monthly aggregates)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                period_start INTEGER NOT NULL,
                period_end INTEGER NOT NULL,
                invocations INTEGER NOT NULL DEFAULT 0,
                bandwidth_bytes INTEGER NOT NULL DEFAULT 0,
                vectors_stored INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, period_start)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Register a new user
    pub async fn register(&self, req: RegisterRequest) -> Result<User> {
        // Validate email
        if !is_valid_email(&req.email) {
            return Err(anyhow!("Invalid email format"));
        }

        // Validate password strength
        if req.password.len() < 8 {
            return Err(anyhow!("Password must be at least 8 characters"));
        }

        // Check if email already exists
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = ?)")
            .bind(req.email.to_lowercase())
            .fetch_one(&self.pool)
            .await?;

        if exists {
            return Err(anyhow!("Email already registered"));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let password_hash = hash_password(&req.password)?;
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, plan, created_at, updated_at)
            VALUES (?, ?, ?, ?, 'free', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(req.email.to_lowercase())
        .bind(&password_hash)
        .bind(&req.name)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // Initialize usage tracking for current period
        let period_start = get_period_start(now);
        let period_end = get_period_end(now);

        sqlx::query(
            r#"
            INSERT INTO user_usage (user_id, period_start, period_end, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(period_start)
        .bind(period_end)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_user(&id).await
    }

    /// Authenticate user and return user if successful
    pub async fn login(&self, req: LoginRequest) -> Result<User> {
        let row = sqlx::query(
            r#"
            SELECT id, email, password_hash, name, plan, stripe_customer_id,
                   email_verified, created_at, updated_at, last_login_at
            FROM users
            WHERE email = ?
            "#,
        )
        .bind(req.email.to_lowercase())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let stored_hash: String = row.get("password_hash");
                if !verify_password(&req.password, &stored_hash) {
                    return Err(anyhow!("Invalid email or password"));
                }

                let user = row_to_user(&row)?;

                // Update last login
                let now = chrono::Utc::now().timestamp();
                sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(&user.id)
                    .execute(&self.pool)
                    .await?;

                Ok(user)
            }
            None => Err(anyhow!("Invalid email or password")),
        }
    }

    /// Get user by ID
    pub async fn get_user(&self, id: &str) -> Result<User> {
        let row = sqlx::query(
            r#"
            SELECT id, email, password_hash, name, plan, stripe_customer_id,
                   email_verified, created_at, updated_at, last_login_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => row_to_user(&row),
            None => Err(anyhow!("User not found")),
        }
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, email, password_hash, name, plan, stripe_customer_id,
                   email_verified, created_at, updated_at, last_login_at
            FROM users
            WHERE email = ?
            "#,
        )
        .bind(email.to_lowercase())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    /// Get user by Stripe customer ID
    pub async fn get_user_by_stripe_customer(
        &self,
        stripe_customer_id: &str,
    ) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT id, email, password_hash, name, plan, stripe_customer_id,
                   email_verified, created_at, updated_at, last_login_at
            FROM users
            WHERE stripe_customer_id = ?
            "#,
        )
        .bind(stripe_customer_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    /// Update user profile
    pub async fn update_user(&self, id: &str, req: UpdateUserRequest) -> Result<User> {
        let now = chrono::Utc::now().timestamp();

        if let Some(name) = &req.name {
            sqlx::query("UPDATE users SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(email) = &req.email {
            if !is_valid_email(email) {
                return Err(anyhow!("Invalid email format"));
            }

            // Check if new email is already taken
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM users WHERE email = ? AND id != ?)",
            )
            .bind(email.to_lowercase())
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            if exists {
                return Err(anyhow!("Email already in use"));
            }

            sqlx::query(
                "UPDATE users SET email = ?, email_verified = 0, updated_at = ? WHERE id = ?",
            )
            .bind(email.to_lowercase())
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        self.get_user(id).await
    }

    /// Change user password
    pub async fn change_password(&self, id: &str, req: ChangePasswordRequest) -> Result<()> {
        // Fetch stored hash
        let stored_hash: Option<String> =
            sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        let stored_hash = stored_hash.ok_or_else(|| anyhow!("User not found"))?;

        // Verify current password
        if !verify_password(&req.current_password, &stored_hash) {
            return Err(anyhow!("Current password is incorrect"));
        }

        if req.new_password.len() < 8 {
            return Err(anyhow!("New password must be at least 8 characters"));
        }

        let new_hash = hash_password(&req.new_password)?;
        let now = chrono::Utc::now().timestamp();

        sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(&new_hash)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        // Revoke all sessions except current (force re-login on other devices)
        sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ?")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update user's plan
    pub async fn update_plan(&self, id: &str, plan: Plan) -> Result<User> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query("UPDATE users SET plan = ?, updated_at = ? WHERE id = ?")
            .bind(plan.as_str())
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        self.get_user(id).await
    }

    /// Update Stripe customer ID
    pub async fn update_stripe_customer(&self, id: &str, stripe_customer_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query("UPDATE users SET stripe_customer_id = ?, updated_at = ? WHERE id = ?")
            .bind(stripe_customer_id)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete user account
    pub async fn delete_user(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get user's current usage
    pub async fn get_usage(&self, user_id: &str) -> Result<UserUsage> {
        let now = chrono::Utc::now().timestamp();
        let period_start = get_period_start(now);

        let row = sqlx::query(
            r#"
            SELECT user_id, period_start, period_end, invocations, bandwidth_bytes, vectors_stored
            FROM user_usage
            WHERE user_id = ? AND period_start = ?
            "#,
        )
        .bind(user_id)
        .bind(period_start)
        .fetch_optional(&self.pool)
        .await?;

        // Get function count
        let functions_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM functions WHERE user_id = ? AND status = 'active'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // Get cron job count
        let cron_jobs_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM scheduled_jobs WHERE user_id = ? AND enabled = 1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        match row {
            Some(row) => Ok(UserUsage {
                user_id: row.get("user_id"),
                period_start: row.get("period_start"),
                period_end: row.get("period_end"),
                invocations: row.get::<i64, _>("invocations") as u64,
                bandwidth_bytes: row.get::<i64, _>("bandwidth_bytes") as u64,
                vectors_stored: row.get::<i64, _>("vectors_stored") as u64,
                functions_count: functions_count as u32,
                cron_jobs_count: cron_jobs_count as u32,
            }),
            None => {
                // Create new usage record
                let period_end = get_period_end(now);
                sqlx::query(
                    r#"
                    INSERT INTO user_usage (user_id, period_start, period_end, updated_at)
                    VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(user_id)
                .bind(period_start)
                .bind(period_end)
                .bind(now)
                .execute(&self.pool)
                .await?;

                Ok(UserUsage {
                    user_id: user_id.to_string(),
                    period_start,
                    period_end,
                    invocations: 0,
                    bandwidth_bytes: 0,
                    vectors_stored: 0,
                    functions_count: functions_count as u32,
                    cron_jobs_count: cron_jobs_count as u32,
                })
            }
        }
    }

    /// Increment usage counters
    pub async fn increment_usage(
        &self,
        user_id: &str,
        invocations: u64,
        bandwidth_bytes: u64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let period_start = get_period_start(now);
        let period_end = get_period_end(now);

        sqlx::query(
            r#"
            INSERT INTO user_usage (user_id, period_start, period_end, invocations, bandwidth_bytes, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, period_start) DO UPDATE SET
                invocations = invocations + excluded.invocations,
                bandwidth_bytes = bandwidth_bytes + excluded.bandwidth_bytes,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(user_id)
        .bind(period_start)
        .bind(period_end)
        .bind(invocations as i64)
        .bind(bandwidth_bytes as i64)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if user is within plan limits
    pub async fn check_limits(&self, user_id: &str) -> Result<LimitCheckResult> {
        let user = self.get_user(user_id).await?;
        let usage = self.get_usage(user_id).await?;
        let limits = user.plan.limits();

        let mut violations = Vec::new();

        if usage.invocations >= limits.invocations_per_month {
            violations.push(LimitViolation::Invocations {
                current: usage.invocations,
                limit: limits.invocations_per_month,
            });
        }

        if usage.functions_count >= limits.max_functions {
            violations.push(LimitViolation::Functions {
                current: usage.functions_count,
                limit: limits.max_functions,
            });
        }

        if usage.cron_jobs_count >= limits.cron_jobs {
            violations.push(LimitViolation::CronJobs {
                current: usage.cron_jobs_count,
                limit: limits.cron_jobs,
            });
        }

        if usage.vectors_stored >= limits.vectors_stored {
            violations.push(LimitViolation::Vectors {
                current: usage.vectors_stored,
                limit: limits.vectors_stored,
            });
        }

        let bandwidth_gb = usage.bandwidth_bytes / (1024 * 1024 * 1024);
        if bandwidth_gb >= limits.bandwidth_gb {
            violations.push(LimitViolation::Bandwidth {
                current_gb: bandwidth_gb,
                limit_gb: limits.bandwidth_gb,
            });
        }

        Ok(LimitCheckResult {
            within_limits: violations.is_empty(),
            violations,
            plan: user.plan,
            limits,
        })
    }
}

/// Result of limit check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitCheckResult {
    pub within_limits: bool,
    pub violations: Vec<LimitViolation>,
    pub plan: Plan,
    pub limits: PlanLimits,
}

/// Types of limit violations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LimitViolation {
    Invocations { current: u64, limit: u64 },
    Functions { current: u32, limit: u32 },
    CronJobs { current: u32, limit: u32 },
    Vectors { current: u64, limit: u64 },
    Bandwidth { current_gb: u64, limit_gb: u64 },
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Hash a password using Argon2id.
///
/// Argon2id is the recommended password hashing algorithm (OWASP). Each hash
/// includes a unique random salt, so the same password produces a different
/// hash every time.
fn hash_password(password: &str) -> Result<String> {
    // Generate a random 16-byte salt using getrandom (OS entropy)
    let mut salt_bytes = [0u8; 16];
    getrandom::fill(&mut salt_bytes)
        .map_err(|e| anyhow!("Failed to generate random salt: {}", e))?;
    let salt =
        SaltString::encode_b64(&salt_bytes).map_err(|e| anyhow!("Failed to encode salt: {}", e))?;
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Failed to hash password: {}", e))?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash.
fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Validate email format with basic checks.
///
/// Performs a simplified validation checking for:
/// - Exactly one `@` symbol
/// - Non-empty local part (before @)
/// - Non-empty domain with at least one dot
///
/// This is intentionally permissive; actual email deliverability is verified
/// by sending a confirmation email during registration.
fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

fn row_to_user(row: &sqlx::sqlite::SqliteRow) -> Result<User> {
    Ok(User {
        id: row.get("id"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        name: row.get("name"),
        plan: Plan::from_str(row.get::<String, _>("plan").as_str())?,
        stripe_customer_id: row.get("stripe_customer_id"),
        email_verified: row.get::<i32, _>("email_verified") == 1,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_login_at: row.get("last_login_at"),
    })
}

/// Get the start of the current billing period (first day of month)
fn get_period_start(timestamp: i64) -> i64 {
    let dt = chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_else(chrono::Utc::now);
    let start = dt
        .with_day(1)
        .unwrap()
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();
    start.timestamp()
}

/// Get the end of the current billing period (last day of month)
fn get_period_end(timestamp: i64) -> i64 {
    let dt = chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_else(chrono::Utc::now);

    // Get first day of next month, then subtract 1 second
    let next_month = if dt.month() == 12 {
        dt.with_year(dt.year() + 1).unwrap().with_month(1).unwrap()
    } else {
        dt.with_month(dt.month() + 1).unwrap()
    };

    let end = next_month
        .with_day(1)
        .unwrap()
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();

    end.timestamp() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_limits() {
        let free = Plan::Free.limits();
        assert_eq!(free.invocations_per_month, 100_000);
        assert_eq!(free.cron_jobs, 0);

        let pro = Plan::Pro.limits();
        assert_eq!(pro.invocations_per_month, 1_000_000);
        assert_eq!(pro.cron_jobs, 10);
    }

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@domain.co.uk"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
    }

    #[test]
    fn test_password_hashing() {
        let hash1 = hash_password("password123");
        let hash2 = hash_password("password123");
        assert_eq!(hash1, hash2);

        let hash3 = hash_password("different");
        assert_ne!(hash1, hash3);
    }
}
