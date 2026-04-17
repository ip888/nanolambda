//! Integration tests for scheduler management
//!
//! Tests cron job creation, updates, deletion, and execution tracking.

use nanolambda_storage::scheduler::{
    CreateScheduledJobRequest, JobRunStatus, SchedulerManager, UpdateScheduledJobRequest,
};

/// Helper to create an in-memory scheduler manager for testing
async fn setup_scheduler_manager() -> SchedulerManager {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create a minimal users table (scheduled_jobs has a foreign key to users)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT,
            plan TEXT NOT NULL DEFAULT 'free',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_login_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    // Insert test users that tests will reference
    let now = chrono::Utc::now().timestamp();
    let test_users = vec![
        ("user-123", "user123@test.com"),
        ("user-456", "user456@test.com"),
        ("user-789", "user789@test.com"),
        ("user-999", "user999@test.com"),
        ("user-get", "userget@test.com"),
        ("user-a", "usera@test.com"),
        ("user-b", "userb@test.com"),
        ("user-update", "userupdate@test.com"),
        ("user-cron", "usercron@test.com"),
        ("user-toggle", "usertoggle@test.com"),
        ("user-delete", "userdelete@test.com"),
        ("user-x", "userx@test.com"),
        ("user-run", "userrun@test.com"),
        ("user-fail", "userfail@test.com"),
        ("user-timeout", "usertimeout@test.com"),
        ("user-history", "userhistory@test.com"),
        ("user-due", "userdue@test.com"),
        ("user-cleanup", "usercleanup@test.com"),
    ];

    for (id, email) in test_users {
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, plan, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(email)
        .bind("hashed_password")
        .bind("free")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("Failed to create test user");
    }

    SchedulerManager::new(pool)
        .await
        .expect("Failed to create SchedulerManager")
}

// =============================================================================
// Job Creation Tests
// =============================================================================

#[tokio::test]
async fn test_create_job_success() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "my-function".to_string(),
        name: "Test Job".to_string(),
        description: Some("Test description".to_string()),
        cron_expression: "0 * * * *".to_string(), // Every hour
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager
        .create_job("user-123", request)
        .await
        .expect("Job creation failed");

    assert!(!job.id.is_empty());
    assert_eq!(job.user_id, "user-123");
    assert_eq!(job.function_name, "my-function");
    assert_eq!(job.name, "Test Job");
    assert_eq!(job.description, Some("Test description".to_string()));
    assert_eq!(job.cron_expression, "0 * * * *");
    assert_eq!(job.timezone, "UTC");
    assert!(job.enabled);
    assert_eq!(job.run_count, 0);
    assert_eq!(job.failure_count, 0);
}

#[tokio::test]
async fn test_create_job_with_payload() {
    let manager = setup_scheduler_manager().await;

    let payload = serde_json::json!({
        "key": "value",
        "count": 42
    });

    let request = CreateScheduledJobRequest {
        function_name: "payload-function".to_string(),
        name: "Payload Job".to_string(),
        description: None,
        cron_expression: "*/5 * * * *".to_string(), // Every 5 minutes
        timezone: "America/New_York".to_string(),
        payload: Some(payload.clone()),
        enabled: true,
    };

    let job = manager
        .create_job("user-456", request)
        .await
        .expect("Job creation failed");

    assert_eq!(job.payload, Some(payload));
    assert_eq!(job.timezone, "America/New_York");
}

#[tokio::test]
async fn test_create_job_disabled() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "disabled-function".to_string(),
        name: "Disabled Job".to_string(),
        description: None,
        cron_expression: "0 0 * * *".to_string(), // Daily
        timezone: "UTC".to_string(),
        payload: None,
        enabled: false,
    };

    let job = manager
        .create_job("user-789", request)
        .await
        .expect("Job creation failed");

    assert!(!job.enabled);
}

#[tokio::test]
async fn test_create_job_invalid_cron_rejected() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "invalid-cron-function".to_string(),
        name: "Invalid Cron Job".to_string(),
        description: None,
        cron_expression: "invalid cron".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let result = manager.create_job("user-999", request).await;
    assert!(result.is_err());
}

// =============================================================================
// Job Retrieval Tests
// =============================================================================

#[tokio::test]
async fn test_get_job_by_id() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "get-function".to_string(),
        name: "Get Test Job".to_string(),
        description: Some("Description".to_string()),
        cron_expression: "0 12 * * *".to_string(),
        timezone: "Europe/London".to_string(),
        payload: None,
        enabled: true,
    };

    let created_job = manager.create_job("user-get", request).await.unwrap();

    let retrieved_job = manager
        .get_job(&created_job.id)
        .await
        .expect("Failed to get job");

    assert_eq!(retrieved_job.id, created_job.id);
    assert_eq!(retrieved_job.name, "Get Test Job");
    assert_eq!(retrieved_job.function_name, "get-function");
}

#[tokio::test]
async fn test_get_nonexistent_job_returns_error() {
    let manager = setup_scheduler_manager().await;

    let result = manager.get_job("nonexistent-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_jobs_for_user() {
    let manager = setup_scheduler_manager().await;

    // Create jobs for user A
    for i in 1..=3 {
        let request = CreateScheduledJobRequest {
            function_name: format!("func-a-{}", i),
            name: format!("Job A{}", i),
            description: None,
            cron_expression: "0 * * * *".to_string(),
            timezone: "UTC".to_string(),
            payload: None,
            enabled: true,
        };
        manager.create_job("user-a", request).await.unwrap();
    }

    // Create jobs for user B
    for i in 1..=2 {
        let request = CreateScheduledJobRequest {
            function_name: format!("func-b-{}", i),
            name: format!("Job B{}", i),
            description: None,
            cron_expression: "0 * * * *".to_string(),
            timezone: "UTC".to_string(),
            payload: None,
            enabled: true,
        };
        manager.create_job("user-b", request).await.unwrap();
    }

    // User A should have 3 jobs
    let user_a_jobs = manager.list_jobs("user-a").await.unwrap();
    assert_eq!(user_a_jobs.len(), 3);

    // User B should have 2 jobs
    let user_b_jobs = manager.list_jobs("user-b").await.unwrap();
    assert_eq!(user_b_jobs.len(), 2);

    // User C (no jobs) should have 0 jobs
    let user_c_jobs = manager.list_jobs("user-c").await.unwrap();
    assert_eq!(user_c_jobs.len(), 0);
}

// =============================================================================
// Job Update Tests
// =============================================================================

#[tokio::test]
async fn test_update_job_name_and_description() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "update-func".to_string(),
        name: "Original Name".to_string(),
        description: Some("Original Description".to_string()),
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-update", request).await.unwrap();

    let update = UpdateScheduledJobRequest {
        name: Some("Updated Name".to_string()),
        description: Some("Updated Description".to_string()),
        cron_expression: None,
        timezone: None,
        payload: None,
        enabled: None,
    };

    let updated = manager
        .update_job(&job.id, "user-update", update)
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description, Some("Updated Description".to_string()));
    // Unchanged fields
    assert_eq!(updated.cron_expression, "0 * * * *");
    assert!(updated.enabled);
}

#[tokio::test]
async fn test_update_job_cron_expression() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "cron-update-func".to_string(),
        name: "Cron Update Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(), // Every hour
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-cron", request).await.unwrap();

    let update = UpdateScheduledJobRequest {
        name: None,
        description: None,
        cron_expression: Some("*/15 * * * *".to_string()), // Every 15 min
        timezone: None,
        payload: None,
        enabled: None,
    };

    let updated = manager
        .update_job(&job.id, "user-cron", update)
        .await
        .unwrap();
    assert_eq!(updated.cron_expression, "*/15 * * * *");
    // next_run_at should be recalculated
    assert!(updated.next_run_at > 0);
}

#[tokio::test]
async fn test_update_job_enable_disable() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "toggle-func".to_string(),
        name: "Toggle Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-toggle", request).await.unwrap();
    assert!(job.enabled);

    // Disable
    let disable_update = UpdateScheduledJobRequest {
        name: None,
        description: None,
        cron_expression: None,
        timezone: None,
        payload: None,
        enabled: Some(false),
    };

    let disabled = manager
        .update_job(&job.id, "user-toggle", disable_update)
        .await
        .unwrap();
    assert!(!disabled.enabled);
    // Note: next_run_at is not cleared when disabled, it's just not used

    // Re-enable
    let enable_update = UpdateScheduledJobRequest {
        name: None,
        description: None,
        cron_expression: None,
        timezone: None,
        payload: None,
        enabled: Some(true),
    };

    let enabled = manager
        .update_job(&job.id, "user-toggle", enable_update)
        .await
        .unwrap();
    assert!(enabled.enabled);
    assert!(enabled.next_run_at > 0); // Re-enabled has next run
}

// =============================================================================
// Job Deletion Tests
// =============================================================================

#[tokio::test]
async fn test_delete_job() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "delete-func".to_string(),
        name: "Delete Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-delete", request).await.unwrap();

    // Delete the job
    manager
        .delete_job(&job.id, "user-delete")
        .await
        .expect("Delete failed");

    // Job should no longer exist
    let result = manager.get_job(&job.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_nonexistent_job_returns_error() {
    let manager = setup_scheduler_manager().await;

    // Deleting a nonexistent job returns an error (get_job fails first)
    let result = manager.delete_job("nonexistent-id", "user-x").await;
    assert!(result.is_err());
}

// =============================================================================
// Job Execution Tests
// =============================================================================

#[tokio::test]
async fn test_start_and_complete_run_success() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "run-func".to_string(),
        name: "Run Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-run", request).await.unwrap();

    // Start a run
    let run_id = manager
        .start_run(&job.id, "request-123")
        .await
        .expect("Start run failed");

    assert!(run_id > 0);

    // Complete the run successfully
    let output = serde_json::json!({"result": "success"});
    manager
        .complete_run(run_id, JobRunStatus::Success, Some(output), None)
        .await
        .expect("Complete run failed");

    // Check that job stats were updated
    let updated_job = manager.get_job(&job.id).await.unwrap();
    assert_eq!(updated_job.run_count, 1);
    assert_eq!(updated_job.failure_count, 0);
    assert!(updated_job.last_run_at.is_some());
}

#[tokio::test]
async fn test_complete_run_with_failure() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "fail-func".to_string(),
        name: "Fail Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-fail", request).await.unwrap();

    // Start and fail a run
    let run_id = manager.start_run(&job.id, "request-fail").await.unwrap();
    manager
        .complete_run(run_id, JobRunStatus::Failed, None, Some("Division by zero"))
        .await
        .unwrap();

    let updated_job = manager.get_job(&job.id).await.unwrap();
    assert_eq!(updated_job.run_count, 1);
    assert_eq!(updated_job.failure_count, 1);
    assert_eq!(updated_job.last_error, Some("Division by zero".to_string()));
}

#[tokio::test]
async fn test_complete_run_with_timeout() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "timeout-func".to_string(),
        name: "Timeout Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-timeout", request).await.unwrap();

    let run_id = manager.start_run(&job.id, "request-timeout").await.unwrap();
    manager
        .complete_run(
            run_id,
            JobRunStatus::Timeout,
            None,
            Some("Function timed out"),
        )
        .await
        .unwrap();

    let updated_job = manager.get_job(&job.id).await.unwrap();
    assert_eq!(updated_job.run_count, 1);
    assert_eq!(updated_job.failure_count, 1); // Timeout counts as failure
}

#[tokio::test]
async fn test_get_run_history() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "history-func".to_string(),
        name: "History Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-history", request).await.unwrap();

    // Create several runs
    for i in 1..=5 {
        let run_id = manager
            .start_run(&job.id, &format!("request-{}", i))
            .await
            .unwrap();

        let status = if i % 2 == 0 {
            JobRunStatus::Success
        } else {
            JobRunStatus::Failed
        };

        manager
            .complete_run(run_id, status, None, None)
            .await
            .unwrap();
    }

    // Get run history (limit 10)
    let runs = manager
        .get_runs(&job.id, 10)
        .await
        .expect("Failed to get runs");

    assert_eq!(runs.len(), 5);
}

// =============================================================================
// Due Jobs Tests
// =============================================================================

#[tokio::test]
async fn test_get_due_jobs() {
    let manager = setup_scheduler_manager().await;

    // Create an enabled job with next_run scheduled soon
    let enabled_request = CreateScheduledJobRequest {
        function_name: "due-enabled".to_string(),
        name: "Enabled Due Job".to_string(),
        description: None,
        cron_expression: "* * * * *".to_string(), // Every minute
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };
    let enabled_job = manager
        .create_job("user-due", enabled_request)
        .await
        .unwrap();

    // Create a disabled job
    let disabled_request = CreateScheduledJobRequest {
        function_name: "due-disabled".to_string(),
        name: "Disabled Due Job".to_string(),
        description: None,
        cron_expression: "* * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: false,
    };
    manager
        .create_job("user-due", disabled_request)
        .await
        .unwrap();

    // Get due jobs - use the job's actual next_run_at + buffer to ensure it's matched
    // The query is: next_run_at <= now, so we need to use a time >= next_run_at
    let check_time = enabled_job.next_run_at + 60; // 60 seconds after scheduled run
    let due_jobs = manager.get_due_jobs(check_time).await.unwrap();

    // Only enabled job should be due
    assert_eq!(due_jobs.len(), 1);
    assert_eq!(due_jobs[0].function_name, "due-enabled");
}

// =============================================================================
// Cleanup Tests
// =============================================================================

#[tokio::test]
async fn test_cleanup_old_runs() {
    let manager = setup_scheduler_manager().await;

    let request = CreateScheduledJobRequest {
        function_name: "cleanup-func".to_string(),
        name: "Cleanup Job".to_string(),
        description: None,
        cron_expression: "0 * * * *".to_string(),
        timezone: "UTC".to_string(),
        payload: None,
        enabled: true,
    };

    let job = manager.create_job("user-cleanup", request).await.unwrap();

    // Create a run
    let run_id = manager.start_run(&job.id, "cleanup-req").await.unwrap();
    manager
        .complete_run(run_id, JobRunStatus::Success, None, None)
        .await
        .unwrap();

    // Verify run exists
    let runs_before = manager.get_runs(&job.id, 10).await.unwrap();
    assert_eq!(runs_before.len(), 1);

    // Cleanup runs using negative days (-1) which makes cutoff = now + 86400 (tomorrow)
    // This ensures all current runs are "older than" the cutoff
    let deleted = manager.cleanup_old_runs(-1).await.unwrap();
    assert_eq!(deleted, 1);

    // Verify runs are gone
    let runs_after = manager.get_runs(&job.id, 10).await.unwrap();
    assert_eq!(runs_after.len(), 0);
}
