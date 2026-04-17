//! User authentication handlers
//!
//! Handles user registration, login, profile management, and session handling.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

use crate::ApiServer;
use nanolambda_storage::user::{
    ChangePasswordRequest, LoginRequest, Plan, RegisterRequest, UpdateUserRequest, User, UserUsage,
};

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub plan: String,
    pub email_verified: bool,
    pub created_at: i64,
    pub usage: Option<UsageResponse>,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub period_start: i64,
    pub period_end: i64,
    pub invocations: u64,
    pub invocations_limit: u64,
    pub bandwidth_bytes: u64,
    pub bandwidth_limit_gb: u64,
    pub vectors_stored: u64,
    pub vectors_limit: u64,
    pub functions_count: u32,
    pub functions_limit: u32,
    pub cron_jobs_count: u32,
    pub cron_jobs_limit: u32,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct LimitsResponse {
    pub within_limits: bool,
    pub plan: String,
    pub violations: Vec<serde_json::Value>,
    pub limits: LimitDetails,
}

#[derive(Debug, Serialize)]
pub struct LimitDetails {
    pub invocations_per_month: u64,
    pub max_memory_mb: u32,
    pub max_timeout_seconds: u32,
    pub max_functions: u32,
    pub bandwidth_gb: u64,
    pub vectors_stored: u64,
    pub cron_jobs: u32,
    pub websocket_connections: u32,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Register a new user
pub async fn register(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> impl IntoResponse {
    let user_manager = match state.get_user_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "User management not available"})),
            );
        }
    };

    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Authentication not available"})),
            );
        }
    };

    match user_manager.register(request).await {
        Ok(user) => {
            info!("User registered: {}", user.email);

            // Generate tokens
            let user_agent = headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let ip_address = headers
                .get("x-forwarded-for")
                .or_else(|| headers.get("x-real-ip"))
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            match jwt_manager
                .generate_tokens(
                    &user.id,
                    &user.email,
                    user.plan.as_str(),
                    user_agent.as_deref(),
                    ip_address.as_deref(),
                )
                .await
            {
                Ok(tokens) => {
                    let response = AuthResponse {
                        user: user_to_response(user, None),
                        access_token: tokens.access_token,
                        refresh_token: tokens.refresh_token,
                        token_type: tokens.token_type,
                        expires_in: tokens.expires_in,
                    };
                    (StatusCode::CREATED, Json(json!(response)))
                }
                Err(e) => {
                    error!("Failed to generate tokens: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to generate tokens"})),
                    )
                }
            }
        }
        Err(e) => {
            let message = e.to_string();
            let status = if message.contains("already registered") {
                StatusCode::CONFLICT
            } else if message.contains("Invalid") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({"error": message})))
        }
    }
}

/// Login user
pub async fn login(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    let user_manager = match state.get_user_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "User management not available"})),
            );
        }
    };

    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Authentication not available"})),
            );
        }
    };

    match user_manager.login(request).await {
        Ok(user) => {
            info!("User logged in: {}", user.email);

            let user_agent = headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let ip_address = headers
                .get("x-forwarded-for")
                .or_else(|| headers.get("x-real-ip"))
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            match jwt_manager
                .generate_tokens(
                    &user.id,
                    &user.email,
                    user.plan.as_str(),
                    user_agent.as_deref(),
                    ip_address.as_deref(),
                )
                .await
            {
                Ok(tokens) => {
                    // Get usage info
                    let usage = user_manager.get_usage(&user.id).await.ok();
                    let usage_response = usage.map(|u| usage_to_response(u, &user.plan));

                    let response = AuthResponse {
                        user: user_to_response(user, usage_response),
                        access_token: tokens.access_token,
                        refresh_token: tokens.refresh_token,
                        token_type: tokens.token_type,
                        expires_in: tokens.expires_in,
                    };
                    (StatusCode::OK, Json(json!(response)))
                }
                Err(e) => {
                    error!("Failed to generate tokens: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to generate tokens"})),
                    )
                }
            }
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Logout user (revoke current session)
pub async fn logout(State(state): State<Arc<ApiServer>>, headers: HeaderMap) -> impl IntoResponse {
    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Authentication not available"})),
            );
        }
    };

    // Extract and validate token
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

    // Revoke all sessions for user
    match jwt_manager.revoke_session(&claims.sub, None).await {
        Ok(_) => {
            info!("User logged out: {}", claims.email);
            (
                StatusCode::OK,
                Json(json!({"message": "Logged out successfully"})),
            )
        }
        Err(e) => {
            error!("Failed to revoke session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to logout"})),
            )
        }
    }
}

/// Refresh access token
pub async fn refresh_token(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Json(request): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Authentication not available"})),
            );
        }
    };

    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    match jwt_manager
        .refresh_tokens(
            &request.refresh_token,
            user_agent.as_deref(),
            ip_address.as_deref(),
        )
        .await
    {
        Ok(tokens) => {
            let response = TokenRefreshResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                token_type: tokens.token_type,
                expires_in: tokens.expires_in,
            };
            (StatusCode::OK, Json(json!(response)))
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Get current user profile
pub async fn get_me(State(state): State<Arc<ApiServer>>, headers: HeaderMap) -> impl IntoResponse {
    let (user_manager, jwt_manager) = match (state.get_user_manager(), state.get_jwt_manager()) {
        (Some(um), Some(jm)) => (um, jm),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service not available"})),
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

    match user_manager.get_user(&claims.sub).await {
        Ok(user) => {
            let usage = user_manager.get_usage(&user.id).await.ok();
            let usage_response = usage.map(|u| usage_to_response(u, &user.plan));
            (
                StatusCode::OK,
                Json(json!(user_to_response(user, usage_response))),
            )
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

/// Update user profile
pub async fn update_me(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Json(request): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let (user_manager, jwt_manager) = match (state.get_user_manager(), state.get_jwt_manager()) {
        (Some(um), Some(jm)) => (um, jm),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service not available"})),
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

    match user_manager.update_user(&claims.sub, request).await {
        Ok(user) => {
            info!("User updated: {}", user.email);
            (StatusCode::OK, Json(json!(user_to_response(user, None))))
        }
        Err(e) => {
            let status = if e.to_string().contains("Invalid") {
                StatusCode::BAD_REQUEST
            } else if e.to_string().contains("already in use") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({"error": e.to_string()})))
        }
    }
}

/// Change password
pub async fn change_password(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let (user_manager, jwt_manager) = match (state.get_user_manager(), state.get_jwt_manager()) {
        (Some(um), Some(jm)) => (um, jm),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service not available"})),
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

    match user_manager.change_password(&claims.sub, request).await {
        Ok(_) => {
            info!("Password changed for user: {}", claims.email);
            (
                StatusCode::OK,
                Json(json!({"message": "Password changed successfully"})),
            )
        }
        Err(e) => {
            let status = if e.to_string().contains("incorrect") {
                StatusCode::UNAUTHORIZED
            } else if e.to_string().contains("at least") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({"error": e.to_string()})))
        }
    }
}

/// Get user's current usage
pub async fn get_usage(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_manager, jwt_manager) = match (state.get_user_manager(), state.get_jwt_manager()) {
        (Some(um), Some(jm)) => (um, jm),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service not available"})),
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

    let user = match user_manager.get_user(&claims.sub).await {
        Ok(u) => u,
        Err(e) => return (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    };

    match user_manager.get_usage(&claims.sub).await {
        Ok(usage) => {
            let response = usage_to_response(usage, &user.plan);
            (StatusCode::OK, Json(json!(response)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Check user's plan limits
pub async fn check_limits(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_manager, jwt_manager) = match (state.get_user_manager(), state.get_jwt_manager()) {
        (Some(um), Some(jm)) => (um, jm),
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Service not available"})),
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

    match user_manager.check_limits(&claims.sub).await {
        Ok(result) => {
            let response = LimitsResponse {
                within_limits: result.within_limits,
                plan: result.plan.as_str().to_string(),
                violations: result
                    .violations
                    .iter()
                    .map(|v| serde_json::to_value(v).unwrap())
                    .collect(),
                limits: LimitDetails {
                    invocations_per_month: result.limits.invocations_per_month,
                    max_memory_mb: result.limits.max_memory_mb,
                    max_timeout_seconds: result.limits.max_timeout_seconds,
                    max_functions: result.limits.max_functions,
                    bandwidth_gb: result.limits.bandwidth_gb,
                    vectors_stored: result.limits.vectors_stored,
                    cron_jobs: result.limits.cron_jobs,
                    websocket_connections: result.limits.websocket_connections,
                },
            };
            (StatusCode::OK, Json(json!(response)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Get active sessions
pub async fn get_sessions(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Authentication not available"})),
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

    match jwt_manager.get_sessions(&claims.sub).await {
        Ok(sessions) => {
            let response: Vec<SessionResponse> = sessions
                .into_iter()
                .map(|s| SessionResponse {
                    id: s.id,
                    user_agent: s.user_agent,
                    ip_address: s.ip_address,
                    created_at: s.created_at,
                    expires_at: s.expires_at,
                })
                .collect();
            (StatusCode::OK, Json(json!({"sessions": response})))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

/// Revoke a specific session
pub async fn revoke_session(
    State(state): State<Arc<ApiServer>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Authentication not available"})),
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

    match jwt_manager
        .revoke_session(&claims.sub, Some(&session_id))
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({"message": "Session revoked"}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    auth_str.strip_prefix("Bearer ").map(|s| s.to_string())
}

fn user_to_response(user: User, usage: Option<UsageResponse>) -> UserResponse {
    UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        plan: user.plan.as_str().to_string(),
        email_verified: user.email_verified,
        created_at: user.created_at,
        usage,
    }
}

fn usage_to_response(usage: UserUsage, plan: &Plan) -> UsageResponse {
    let limits = plan.limits();
    UsageResponse {
        period_start: usage.period_start,
        period_end: usage.period_end,
        invocations: usage.invocations,
        invocations_limit: limits.invocations_per_month,
        bandwidth_bytes: usage.bandwidth_bytes,
        bandwidth_limit_gb: limits.bandwidth_gb,
        vectors_stored: usage.vectors_stored,
        vectors_limit: limits.vectors_stored,
        functions_count: usage.functions_count,
        functions_limit: limits.max_functions,
        cron_jobs_count: usage.cron_jobs_count,
        cron_jobs_limit: limits.cron_jobs,
    }
}
