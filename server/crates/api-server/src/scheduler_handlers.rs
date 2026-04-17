//! Scheduled jobs (cron) handlers
//!
//! Handles CRUD operations for scheduled function executions.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

use crate::ApiServer;
use nanolambda_storage::scheduler::{
    CreateScheduledJobRequest, JobRun, ScheduledJob, UpdateScheduledJobRequest,
};

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ScheduledJobResponse {
    pub id: String,
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
    pub run_count: i64,
    pub failure_count: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JobRunResponse {
    pub id: i64,
    pub request_id: String,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListRunsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Create a new scheduled job
pub async fn create_schedule(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Json(request): Json<CreateScheduledJobRequest>,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    // Authenticate user
    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    // Check plan limits
    if let Some(user_manager) = state.get_user_manager() {
        match user_manager.check_limits(&claims.sub).await {
            Ok(result) => {
                if !result.within_limits {
                    for violation in &result.violations {
                        // Check if it's a CronJobs violation using pattern matching
                        if let Ok(serde_json::Value::Object(v)) = serde_json::to_value(violation)
                            && v.get("type").and_then(|t| t.as_str()) == Some("CronJobs")
                        {
                            return (
                                StatusCode::PAYMENT_REQUIRED,
                                Json(json!({
                                    "error": "Cron job limit reached",
                                    "message": "Please upgrade your plan to create more scheduled jobs",
                                    "current_plan": result.plan.as_str()
                                })),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to check limits: {}", e);
            }
        }
    }

    // Verify function exists
    let func_result = state.storage().get_function(&request.function_name);
    if func_result.is_err() || func_result.as_ref().unwrap().is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!("Function '{}' not found", request.function_name)})),
        );
    }

    match scheduler.create_job(&claims.sub, request).await {
        Ok(job) => {
            info!(
                "Scheduled job created: {} for function {}",
                job.id, job.function_name
            );
            (StatusCode::CREATED, Json(json!(job_to_response(job))))
        }
        Err(e) => {
            let status = if e.to_string().contains("Invalid") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({"error": e.to_string()})))
        }
    }
}

/// List scheduled jobs for current user
pub async fn list_schedules(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    match scheduler.list_jobs(&claims.sub).await {
        Ok(jobs) => {
            let response: Vec<ScheduledJobResponse> =
                jobs.into_iter().map(job_to_response).collect();
            (
                StatusCode::OK,
                Json(json!({
                    "schedules": response,
                    "count": response.len()
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Get a specific scheduled job
pub async fn get_schedule(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    match scheduler.get_job(&id).await {
        Ok(job) => {
            if job.user_id != claims.sub {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Access denied"})),
                );
            }
            (StatusCode::OK, Json(json!(job_to_response(job))))
        }
        Err(e) => {
            if e.to_string().contains("not found") {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "Schedule not found"})),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
            }
        }
    }
}

/// Update a scheduled job
pub async fn update_schedule(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateScheduledJobRequest>,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    match scheduler.update_job(&id, &claims.sub, request).await {
        Ok(job) => {
            info!("Scheduled job updated: {}", job.id);
            (StatusCode::OK, Json(json!(job_to_response(job))))
        }
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("Unauthorized") {
                StatusCode::FORBIDDEN
            } else if e.to_string().contains("Invalid") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({"error": e.to_string()})))
        }
    }
}

/// Delete a scheduled job
pub async fn delete_schedule(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    match scheduler.delete_job(&id, &claims.sub).await {
        Ok(_) => {
            info!("Scheduled job deleted: {}", id);
            (StatusCode::OK, Json(json!({"message": "Schedule deleted"})))
        }
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("Unauthorized") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({"error": e.to_string()})))
        }
    }
}

/// Get execution history for a scheduled job
pub async fn get_schedule_runs(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ListRunsQuery>,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    // Verify ownership
    match scheduler.get_job(&id).await {
        Ok(job) => {
            if job.user_id != claims.sub {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Access denied"})),
                );
            }
        }
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Schedule not found"})),
            );
        }
    }

    match scheduler.get_runs(&id, query.limit.min(100)).await {
        Ok(runs) => {
            let response: Vec<JobRunResponse> = runs.into_iter().map(run_to_response).collect();
            (
                StatusCode::OK,
                Json(json!({
                    "runs": response,
                    "count": response.len()
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Trigger a scheduled job immediately (manual run)
pub async fn trigger_schedule(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let (scheduler, jwt_manager) = match (state.get_scheduler_manager(), state.get_jwt_manager()) {
        (Some(s), Some(j)) => (s, j),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Scheduler not available"})),
            );
        }
    };

    let token = match extract_bearer_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization token"})),
            );
        }
    };

    let claims = match jwt_manager.validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            );
        }
    };

    // Get and verify ownership
    let job = match scheduler.get_job(&id).await {
        Ok(job) => {
            if job.user_id != claims.sub {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Access denied"})),
                );
            }
            job
        }
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Schedule not found"})),
            );
        }
    };

    // Execute the function
    let request_id = uuid::Uuid::new_v4().to_string();

    // Start run record
    let run_id = match scheduler.start_run(&id, &request_id).await {
        Ok(rid) => rid,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to start run: {}", e)})),
            );
        }
    };

    info!(
        "Manual trigger of scheduled job: {} (run_id: {})",
        id, run_id
    );

    // Execute function asynchronously
    let state_clone = state.clone();
    let _job_id = id.clone();
    let scheduler_clone = scheduler.clone();

    tokio::spawn(async move {
        let result = state_clone
            .invoke_function_by_name(&job.function_name, job.payload.clone())
            .await;

        match result {
            Ok(output) => {
                let _ = scheduler_clone
                    .complete_run(
                        run_id,
                        nanolambda_storage::scheduler::JobRunStatus::Success,
                        Some(output),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let _ = scheduler_clone
                    .complete_run(
                        run_id,
                        nanolambda_storage::scheduler::JobRunStatus::Failed,
                        None,
                        Some(&e.to_string()),
                    )
                    .await;
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "message": "Job triggered",
            "request_id": request_id,
            "run_id": run_id
        })),
    )
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    auth_str.strip_prefix("Bearer ").map(|s| s.to_string())
}

fn job_to_response(job: ScheduledJob) -> ScheduledJobResponse {
    ScheduledJobResponse {
        id: job.id,
        function_name: job.function_name,
        name: job.name,
        description: job.description,
        cron_expression: job.cron_expression,
        timezone: job.timezone,
        payload: job.payload,
        enabled: job.enabled,
        last_run_at: job.last_run_at,
        next_run_at: job.next_run_at,
        created_at: job.created_at,
        run_count: job.run_count,
        failure_count: job.failure_count,
        last_error: job.last_error,
    }
}

fn run_to_response(run: JobRun) -> JobRunResponse {
    JobRunResponse {
        id: run.id,
        request_id: run.request_id,
        status: run.status.as_str().to_string(),
        started_at: run.started_at,
        completed_at: run.completed_at,
        duration_ms: run.duration_ms,
        error: run.error,
    }
}
