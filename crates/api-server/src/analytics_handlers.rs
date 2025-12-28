// Usage Analytics API Handlers
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{ApiServer, handlers::ErrorResponse};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetUsageSnapshotRequest {
    pub period_type: String,
    pub invocations: i64,
    pub execution_hours: f64,
    pub errors: i64,
}

/// Get daily usage summaries for the past N days
pub async fn get_daily_summaries(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing or invalid x-api-key header".to_string(),
            }),
        ))?;

    let manager = state.analytics_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal error".to_string(),
            message: "Analytics manager not available".to_string(),
        }),
    ))?;

    let summaries = manager
        .get_daily_summaries(api_key_str, 30)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve data".to_string(),
                    message: "Failed to retrieve daily summaries".to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": summaries,
    })))
}

/// Get usage profile for current customer
pub async fn get_usage_profile(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing or invalid x-api-key header".to_string(),
            }),
        ))?;

    let manager = state.analytics_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal error".to_string(),
            message: "Analytics manager not available".to_string(),
        }),
    ))?;

    let profile = manager
        .calculate_usage_profile(api_key_str, "pro", 30, 10000, 9500)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve data".to_string(),
                    message: "Failed to calculate usage profile".to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": profile,
    })))
}

/// Get usage analytics snapshot
pub async fn get_usage_snapshot(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<GetUsageSnapshotRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing or invalid x-api-key header".to_string(),
            }),
        ))?;

    let manager = state.analytics_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal error".to_string(),
            message: "Analytics manager not available".to_string(),
        }),
    ))?;

    let functions = vec![
        ("handler".to_string(), 1000),
        ("processor".to_string(), 500),
    ];

    let snapshot = manager
        .get_usage_analytics_snapshot(
            api_key_str,
            &req.period_type,
            req.invocations,
            req.execution_hours,
            req.errors,
            functions,
        )
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve data".to_string(),
                    message: "Failed to get usage snapshot".to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": snapshot,
    })))
}

/// Get platform analytics summary (admin endpoint)
pub async fn get_platform_analytics(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let manager = state.analytics_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal error".to_string(),
            message: "Analytics manager not available".to_string(),
        }),
    ))?;

    let summary = manager
        .get_platform_analytics_summary()
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve data".to_string(),
                    message: "Failed to retrieve platform analytics".to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": summary,
    })))
}

/// Get monthly trends for a customer
pub async fn get_monthly_trends(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing or invalid x-api-key header".to_string(),
            }),
        ))?;

    let manager = state.analytics_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal error".to_string(),
            message: "Analytics manager not available".to_string(),
        }),
    ))?;

    let trends = manager.get_monthly_trends(api_key_str).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve data".to_string(),
                message: "Failed to retrieve monthly trends".to_string(),
            }),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "data": trends,
    })))
}
