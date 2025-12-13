use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::ApiServer;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RecordPaymentFailureRequest {
    pub amount_cents: i64,
    pub failure_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ClearRetryRequest {
    pub resolution_note: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /payment-retry/record-failure
/// Record a payment failure and initiate retry process
pub async fn record_payment_failure(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RecordPaymentFailureRequest>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Extract API key from header
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "missing_api_key".to_string(),
                    message: "x-api-key header is required".to_string(),
                }),
            )
        })?;

    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    let status = retry_manager
        .record_payment_failure(
            api_key_str,
            payload.amount_cents,
            &payload.failure_reason,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "record_failure_failed".to_string(),
                    message: format!("Failed to record payment failure: {}", e),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": status
    })))
}

/// POST /payment-retry/process
/// Process a retry attempt for customer
pub async fn process_retry(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "missing_api_key".to_string(),
                    message: "x-api-key header is required".to_string(),
                }),
            )
        })?;

    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    let status = retry_manager
        .process_retry(api_key_str)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "retry_failed".to_string(),
                    message: format!("{}", e),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": status
    })))
}

/// GET /payment-retry/status
/// Get retry status for customer
pub async fn get_retry_status(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "missing_api_key".to_string(),
                    message: "x-api-key header is required".to_string(),
                }),
            )
        })?;

    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    let status = retry_manager
        .get_retry_status(api_key_str)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "get_status_failed".to_string(),
                    message: format!("Failed to get retry status: {}", e),
                }),
            )
        })?;

    if let Some(status) = status {
        Ok(Json(json!({
            "success": true,
            "data": status
        })))
    } else {
        Ok(Json(json!({
            "success": true,
            "data": null,
            "message": "No retry status found - account is current"
        })))
    }
}

/// POST /payment-retry/clear
/// Clear retry status (manual resolution)
pub async fn clear_retry_status(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(_payload): Json<ClearRetryRequest>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "missing_api_key".to_string(),
                    message: "x-api-key header is required".to_string(),
                }),
            )
        })?;

    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    retry_manager
        .clear_retry_status(api_key_str)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "clear_failed".to_string(),
                    message: format!("Failed to clear retry status: {}", e),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": "Retry status cleared successfully"
    })))
}

/// POST /payment-retry/send-dunning
/// Send dunning notification to customer
pub async fn send_dunning_notification(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "missing_api_key".to_string(),
                    message: "x-api-key header is required".to_string(),
                }),
            )
        })?;

    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    let notification_details = retry_manager
        .send_dunning_notification(api_key_str)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "dunning_failed".to_string(),
                    message: format!("Failed to send dunning notification: {}", e),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": notification_details
    })))
}

/// GET /payment-retry/metrics (PUBLIC)
/// Get platform-wide payment retry metrics
pub async fn get_platform_metrics(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    let metrics = retry_manager
        .get_platform_metrics()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "get_metrics_failed".to_string(),
                    message: format!("Failed to get platform metrics: {}", e),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": metrics
    })))
}

/// GET /payment-retry/past-due (PUBLIC)
/// Get all customers in past due status
pub async fn get_past_due_customers(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    let retry_manager = state.payment_retry_manager().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "payment_retry_unavailable".to_string(),
                message: "Payment retry manager is not available".to_string(),
            }),
        )
    })?;

    let customers = retry_manager
        .get_past_due_customers()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "get_past_due_failed".to_string(),
                    message: format!("Failed to get past due customers: {}", e),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": customers
    })))
}
