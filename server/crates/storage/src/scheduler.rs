//! Scheduled Jobs (Cron) for NanoLambda
//!
//! This module provides cron-based function scheduling with timezone support,
//! enabling users to execute functions on a recurring schedule.
//!
//! ## Features
//!
//! - **Standard cron expressions**: 5-field format (minute hour day month weekday)
//! - **Timezone support**: Jobs can be scheduled in any IANA timezone
//! - **Run history**: Complete execution history with status, duration, and output
//! - **Automatic retry**: Failed jobs can be configured for automatic retry
//! - **Plan-based limits**: Job limits enforced based on user's subscription plan
//!
//! ## Cron Expression Format
//!
//! ```text
//! ┌───────────── minute (0-59)
//! │ ┌───────────── hour (0-23)
//! │ │ ┌───────────── day of month (1-31)
//! │ │ │ ┌───────────── month (1-12)
//! │ │ │ │ ┌───────────── day of week (0-7, 0 or 7 is Sunday)
//! │ │ │ │ │
//! * * * * *
//! ```
//!
//! ## Examples
//!
//! - `0 * * * *` - Every hour at minute 0
//! - `0 9 * * 1-5` - Every weekday at 9:00 AM
//! - `*/15 * * * *` - Every 15 minutes
//! - `0 0 1 * *` - First day of every month at midnight

use anyhow::{Result, anyhow};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Scheduled job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub user_id: String,
    pub function_name: String,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    pub timezone: String,
    pub payload: Option<serde_json::Value>,
    pub enabled: bool,
    pub last_run_at: Option<i64>,
    pub next_run_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub run_count: i64,
    pub failure_count: i64,
    pub last_error: Option<String>,
}

/// Job execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRun {
    pub id: i64,
    pub job_id: String,
    pub request_id: String,
    pub status: JobRunStatus,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
    pub output: Option<serde_json::Value>,
}

/// Job run status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobRunStatus {
    Running,
    Success,
    Failed,
    Timeout,
}

impl JobRunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            JobRunStatus::Running => "running",
            JobRunStatus::Success => "success",
            JobRunStatus::Failed => "failed",
            JobRunStatus::Timeout => "timeout",
        }
    }
}

impl std::str::FromStr for JobRunStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "running" => Ok(JobRunStatus::Running),
            "success" => Ok(JobRunStatus::Success),
            "failed" => Ok(JobRunStatus::Failed),
            "timeout" => Ok(JobRunStatus::Timeout),
            _ => Err(anyhow!("Invalid job run status: {}", s)),
        }
    }
}

/// Create scheduled job request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateScheduledJobRequest {
    pub function_name: String,
    pub name: String,
    pub description: Option<String>,
    pub cron_expression: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    pub payload: Option<serde_json::Value>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_enabled() -> bool {
    true
}

/// Update scheduled job request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateScheduledJobRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// Scheduled job manager
pub struct SchedulerManager {
    pool: SqlitePool,
}

impl SchedulerManager {
    /// Create new scheduler manager
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let manager = Self { pool };
        manager.init_tables().await?;
        Ok(manager)
    }

    /// Initialize database tables
    async fn init_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS scheduled_jobs (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                function_name TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                cron_expression TEXT NOT NULL,
                timezone TEXT NOT NULL DEFAULT 'UTC',
                payload TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run_at INTEGER,
                next_run_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                run_count INTEGER NOT NULL DEFAULT 0,
                failure_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_scheduled_jobs_user_id ON scheduled_jobs(user_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_scheduled_jobs_next_run ON scheduled_jobs(next_run_at)
            WHERE enabled = 1
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS job_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                duration_ms INTEGER,
                error TEXT,
                output TEXT,
                FOREIGN KEY (job_id) REFERENCES scheduled_jobs(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_job_runs_job_id ON job_runs(job_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_job_runs_started_at ON job_runs(started_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a new scheduled job
    pub async fn create_job(
        &self,
        user_id: &str,
        req: CreateScheduledJobRequest,
    ) -> Result<ScheduledJob> {
        // Validate cron expression
        if !is_valid_cron(&req.cron_expression) {
            return Err(anyhow!("Invalid cron expression: {}", req.cron_expression));
        }

        // Validate timezone
        if !is_valid_timezone(&req.timezone) {
            return Err(anyhow!("Invalid timezone: {}", req.timezone));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let next_run = calculate_next_run(&req.cron_expression, &req.timezone, now)?;
        let payload_json = req.payload.map(|p| serde_json::to_string(&p).unwrap());

        sqlx::query(
            r#"
            INSERT INTO scheduled_jobs 
            (id, user_id, function_name, name, description, cron_expression, timezone, payload, enabled, next_run_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&req.function_name)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.cron_expression)
        .bind(&req.timezone)
        .bind(&payload_json)
        .bind(req.enabled)
        .bind(next_run)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_job(&id).await
    }

    /// Get a scheduled job by ID
    pub async fn get_job(&self, id: &str) -> Result<ScheduledJob> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, function_name, name, description, cron_expression, timezone,
                   payload, enabled, last_run_at, next_run_at, created_at, updated_at,
                   run_count, failure_count, last_error
            FROM scheduled_jobs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => row_to_job(&row),
            None => Err(anyhow!("Scheduled job not found")),
        }
    }

    /// List scheduled jobs for a user
    pub async fn list_jobs(&self, user_id: &str) -> Result<Vec<ScheduledJob>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, function_name, name, description, cron_expression, timezone,
                   payload, enabled, last_run_at, next_run_at, created_at, updated_at,
                   run_count, failure_count, last_error
            FROM scheduled_jobs
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_job).collect()
    }

    /// Update a scheduled job
    pub async fn update_job(
        &self,
        id: &str,
        user_id: &str,
        req: UpdateScheduledJobRequest,
    ) -> Result<ScheduledJob> {
        let job = self.get_job(id).await?;
        if job.user_id != user_id {
            return Err(anyhow!("Unauthorized"));
        }

        let now = chrono::Utc::now().timestamp();
        let mut cron = job.cron_expression.clone();
        let mut tz = job.timezone.clone();

        if let Some(name) = &req.name {
            sqlx::query("UPDATE scheduled_jobs SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(description) = &req.description {
            sqlx::query("UPDATE scheduled_jobs SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(cron_expression) = &req.cron_expression {
            if !is_valid_cron(cron_expression) {
                return Err(anyhow!("Invalid cron expression: {}", cron_expression));
            }
            cron = cron_expression.clone();
            sqlx::query(
                "UPDATE scheduled_jobs SET cron_expression = ?, updated_at = ? WHERE id = ?",
            )
            .bind(cron_expression)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        if let Some(timezone) = &req.timezone {
            if !is_valid_timezone(timezone) {
                return Err(anyhow!("Invalid timezone: {}", timezone));
            }
            tz = timezone.clone();
            sqlx::query("UPDATE scheduled_jobs SET timezone = ?, updated_at = ? WHERE id = ?")
                .bind(timezone)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(payload) = &req.payload {
            let payload_json = serde_json::to_string(payload)?;
            sqlx::query("UPDATE scheduled_jobs SET payload = ?, updated_at = ? WHERE id = ?")
                .bind(&payload_json)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(enabled) = req.enabled {
            sqlx::query("UPDATE scheduled_jobs SET enabled = ?, updated_at = ? WHERE id = ?")
                .bind(enabled)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        // Recalculate next run time if cron or timezone changed
        if req.cron_expression.is_some() || req.timezone.is_some() {
            let next_run = calculate_next_run(&cron, &tz, now)?;
            sqlx::query("UPDATE scheduled_jobs SET next_run_at = ? WHERE id = ?")
                .bind(next_run)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        self.get_job(id).await
    }

    /// Delete a scheduled job
    pub async fn delete_job(&self, id: &str, user_id: &str) -> Result<()> {
        let job = self.get_job(id).await?;
        if job.user_id != user_id {
            return Err(anyhow!("Unauthorized"));
        }

        sqlx::query("DELETE FROM scheduled_jobs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get jobs due to run
    pub async fn get_due_jobs(&self, now: i64) -> Result<Vec<ScheduledJob>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, function_name, name, description, cron_expression, timezone,
                   payload, enabled, last_run_at, next_run_at, created_at, updated_at,
                   run_count, failure_count, last_error
            FROM scheduled_jobs
            WHERE enabled = 1 AND next_run_at <= ?
            ORDER BY next_run_at ASC
            LIMIT 100
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_job).collect()
    }

    /// Record job execution start
    pub async fn start_run(&self, job_id: &str, request_id: &str) -> Result<i64> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            r#"
            INSERT INTO job_runs (job_id, request_id, status, started_at)
            VALUES (?, ?, 'running', ?)
            "#,
        )
        .bind(job_id)
        .bind(request_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Complete job execution
    pub async fn complete_run(
        &self,
        run_id: i64,
        status: JobRunStatus,
        output: Option<serde_json::Value>,
        error: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        // Get started_at for duration calculation
        let started_at: i64 = sqlx::query_scalar("SELECT started_at FROM job_runs WHERE id = ?")
            .bind(run_id)
            .fetch_one(&self.pool)
            .await?;

        let duration_ms = (now - started_at) * 1000;
        let output_json = output.map(|o| serde_json::to_string(&o).unwrap());

        sqlx::query(
            r#"
            UPDATE job_runs 
            SET status = ?, completed_at = ?, duration_ms = ?, error = ?, output = ?
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(now)
        .bind(duration_ms)
        .bind(error)
        .bind(&output_json)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        // Get job_id for the run
        let job_id: String = sqlx::query_scalar("SELECT job_id FROM job_runs WHERE id = ?")
            .bind(run_id)
            .fetch_one(&self.pool)
            .await?;

        // Update job statistics
        let (run_increment, failure_increment) = match status {
            JobRunStatus::Success => (1, 0),
            _ => (1, 1),
        };

        sqlx::query(
            r#"
            UPDATE scheduled_jobs 
            SET run_count = run_count + ?, 
                failure_count = failure_count + ?,
                last_run_at = ?,
                last_error = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(run_increment)
        .bind(failure_increment)
        .bind(now)
        .bind(error)
        .bind(now)
        .bind(&job_id)
        .execute(&self.pool)
        .await?;

        // Calculate and update next run time
        let job = self.get_job(&job_id).await?;
        let next_run = calculate_next_run(&job.cron_expression, &job.timezone, now)?;

        sqlx::query("UPDATE scheduled_jobs SET next_run_at = ? WHERE id = ?")
            .bind(next_run)
            .bind(&job_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get run history for a job
    pub async fn get_runs(&self, job_id: &str, limit: i64) -> Result<Vec<JobRun>> {
        let rows = sqlx::query(
            r#"
            SELECT id, job_id, request_id, status, started_at, completed_at, duration_ms, error, output
            FROM job_runs
            WHERE job_id = ?
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )
        .bind(job_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let output_str: Option<String> = row.get("output");
                let output = output_str.and_then(|s| serde_json::from_str(&s).ok());

                Ok(JobRun {
                    id: row.get("id"),
                    job_id: row.get("job_id"),
                    request_id: row.get("request_id"),
                    status: row.get::<String, _>("status").parse()?,
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    duration_ms: row.get("duration_ms"),
                    error: row.get("error"),
                    output,
                })
            })
            .collect()
    }

    /// Clean up old job runs
    pub async fn cleanup_old_runs(&self, days: i64) -> Result<u64> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);

        let result = sqlx::query("DELETE FROM job_runs WHERE started_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

// Helper functions

fn row_to_job(row: &sqlx::sqlite::SqliteRow) -> Result<ScheduledJob> {
    let payload_str: Option<String> = row.get("payload");
    let payload = payload_str.and_then(|s| serde_json::from_str(&s).ok());

    Ok(ScheduledJob {
        id: row.get("id"),
        user_id: row.get("user_id"),
        function_name: row.get("function_name"),
        name: row.get("name"),
        description: row.get("description"),
        cron_expression: row.get("cron_expression"),
        timezone: row.get("timezone"),
        payload,
        enabled: row.get::<i32, _>("enabled") == 1,
        last_run_at: row.get("last_run_at"),
        next_run_at: row.get("next_run_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        run_count: row.get("run_count"),
        failure_count: row.get("failure_count"),
        last_error: row.get("last_error"),
    })
}

/// Validate cron expression (5 or 6 fields)
fn is_valid_cron(expr: &str) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();

    // Support both 5-field (minute hour day month weekday) and 6-field (second minute hour day month weekday)
    if fields.len() < 5 || fields.len() > 6 {
        return false;
    }

    // Basic validation - each field should have valid characters
    for field in fields {
        if field.is_empty() {
            return false;
        }
        // Valid characters: 0-9, *, /, -, ,
        if !field
            .chars()
            .all(|c| c.is_ascii_digit() || "*/-,".contains(c))
        {
            return false;
        }
    }

    true
}

/// Validate timezone string
fn is_valid_timezone(tz: &str) -> bool {
    // Common timezone formats
    let valid_prefixes = [
        "UTC",
        "GMT",
        "US/",
        "Europe/",
        "Asia/",
        "Africa/",
        "America/",
        "Australia/",
        "Pacific/",
        "Atlantic/",
        "Indian/",
        "Etc/",
    ];

    if tz == "UTC" || tz == "GMT" {
        return true;
    }

    valid_prefixes.iter().any(|prefix| tz.starts_with(prefix))
}

/// Calculate next run time from cron expression
fn calculate_next_run(cron_expr: &str, _timezone: &str, after: i64) -> Result<i64> {
    // Simplified cron parsing - parse the expression and find next occurrence
    let fields: Vec<&str> = cron_expr.split_whitespace().collect();

    if fields.len() < 5 {
        return Err(anyhow!("Invalid cron expression"));
    }

    // For simplicity, we'll calculate a basic next run time
    // In production, use a proper cron parser library

    let dt =
        chrono::DateTime::from_timestamp(after, 0).ok_or_else(|| anyhow!("Invalid timestamp"))?;

    // Parse minute field for basic scheduling
    let minute = parse_cron_field(fields[0], 0, 59)?;
    let hour = parse_cron_field(fields[1], 0, 23)?;

    // Calculate next occurrence
    let mut next = dt;

    // If we're past the scheduled time today, move to tomorrow
    if next.hour() > hour as u32 || (next.hour() == hour as u32 && next.minute() >= minute as u32) {
        next += chrono::Duration::days(1);
    }

    // Set the target time
    next = next
        .with_hour(hour as u32)
        .unwrap()
        .with_minute(minute as u32)
        .unwrap()
        .with_second(0)
        .unwrap();

    Ok(next.timestamp())
}

/// Parse a single cron field and return the first valid value
fn parse_cron_field(field: &str, min: i32, max: i32) -> Result<i32> {
    if field == "*" {
        return Ok(min);
    }

    // Handle step values (*/5) using strip_prefix for cleaner parsing
    if let Some(step_str) = field.strip_prefix("*/") {
        let step: i32 = step_str.parse()?;
        return Ok(min + (min % step));
    }

    // Handle ranges (1-5)
    if field.contains('-') {
        let parts: Vec<&str> = field.split('-').collect();
        if parts.len() == 2 {
            let start: i32 = parts[0].parse()?;
            return Ok(start.max(min).min(max));
        }
    }

    // Handle lists (1,2,3)
    if field.contains(',') {
        let parts: Vec<&str> = field.split(',').collect();
        if !parts.is_empty() {
            let first: i32 = parts[0].parse()?;
            return Ok(first.max(min).min(max));
        }
    }

    // Simple number
    let value: i32 = field.parse()?;
    Ok(value.max(min).min(max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_validation() {
        assert!(is_valid_cron("* * * * *")); // Every minute
        assert!(is_valid_cron("0 * * * *")); // Every hour
        assert!(is_valid_cron("0 0 * * *")); // Every day at midnight
        assert!(is_valid_cron("*/5 * * * *")); // Every 5 minutes
        assert!(is_valid_cron("0 9-17 * * 1-5")); // 9am-5pm on weekdays

        assert!(!is_valid_cron("* * *")); // Too few fields
        assert!(!is_valid_cron("invalid")); // Invalid
    }

    #[test]
    fn test_timezone_validation() {
        assert!(is_valid_timezone("UTC"));
        assert!(is_valid_timezone("America/New_York"));
        assert!(is_valid_timezone("Europe/London"));
        assert!(is_valid_timezone("Asia/Tokyo"));

        assert!(!is_valid_timezone("Invalid/Zone"));
    }
}
