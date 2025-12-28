// CLV API Handlers
// HTTP endpoints for Customer Lifetime Value metrics

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::ApiServer;
use crate::handlers::ErrorResponse;

/// Request body for calculating CLV
#[derive(Debug, Deserialize)]
pub struct CalculateCLVRequest {
    pub tier: String,
    pub account_age_days: i64,
    pub total_revenue: i64,
    pub monthly_revenues: Vec<i64>,
    pub retention_probability: f64,
}

/// Request body for revenue prediction
#[derive(Debug, Deserialize)]
pub struct PredictRevenueRequest {
    pub current_monthly_revenue: i64,
    pub growth_rate: f64,
    pub retention_probability: f64,
}

/// Get CLV for authenticated customer
pub async fn get_customer_clv(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Extract API key
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing x-api-key header".to_string(),
            }),
        ))?;

    // Get CLV manager
    let manager = state.clv_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "CLV manager not available".to_string(),
        }),
    ))?;

    // Get CLV data
    let clv = manager.get_customer_clv(api_key_str).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve CLV".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    if let Some(clv_data) = clv {
        Ok(Json(json!({
            "success": true,
            "data": clv_data
        })))
    } else {
        // Return default/empty CLV if not calculated yet
        Ok(Json(json!({
            "success": true,
            "data": {
                "message": "CLV not yet calculated. Use POST /clv/calculate to compute.",
                "api_key": api_key_str
            }
        })))
    }
}

/// Calculate CLV for authenticated customer
pub async fn calculate_clv(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CalculateCLVRequest>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Extract API key
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing x-api-key header".to_string(),
            }),
        ))?;

    // Validate retention probability
    if payload.retention_probability < 0.0 || payload.retention_probability > 1.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid input".to_string(),
                message: "Retention probability must be between 0 and 1".to_string(),
            }),
        ));
    }

    // Get CLV manager
    let manager = state.clv_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "CLV manager not available".to_string(),
        }),
    ))?;

    // Calculate CLV
    let clv = manager
        .calculate_clv(
            api_key_str,
            &payload.tier,
            payload.account_age_days,
            payload.total_revenue,
            payload.monthly_revenues,
            payload.retention_probability,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Calculation failed".to_string(),
                    message: e.to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": clv
    })))
}

/// Get revenue prediction for customer
pub async fn get_revenue_prediction(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<PredictRevenueRequest>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Extract API key
    let api_key_str = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid request".to_string(),
                message: "Missing x-api-key header".to_string(),
            }),
        ))?;

    // Validate inputs
    if payload.retention_probability < 0.0 || payload.retention_probability > 1.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid input".to_string(),
                message: "Retention probability must be between 0 and 1".to_string(),
            }),
        ));
    }

    if payload.growth_rate < -1.0 || payload.growth_rate > 1.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid input".to_string(),
                message: "Growth rate must be between -1 and 1".to_string(),
            }),
        ));
    }

    // Get CLV manager
    let manager = state.clv_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "CLV manager not available".to_string(),
        }),
    ))?;

    // Generate prediction
    let prediction = manager
        .predict_revenue(
            api_key_str,
            payload.current_monthly_revenue,
            payload.growth_rate,
            payload.retention_probability,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Prediction failed".to_string(),
                    message: e.to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": prediction
    })))
}

/// Get CLV segments breakdown
pub async fn get_clv_segments(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Get CLV manager
    let manager = state.clv_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "CLV manager not available".to_string(),
        }),
    ))?;

    // Get segments
    let segments = manager.get_clv_segments().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve segments".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "data": segments
    })))
}

/// Get platform CLV summary (public endpoint)
pub async fn get_platform_clv_summary(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Get CLV manager
    let manager = state.clv_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "CLV manager not available".to_string(),
        }),
    ))?;

    // Get summary
    let summary = manager.get_platform_clv_summary().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve summary".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "data": summary
    })))
}

/// Get cohort analysis
pub async fn get_cohort_analysis(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Get CLV manager
    let manager = state.clv_manager().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "CLV manager not available".to_string(),
        }),
    ))?;

    // Get cohorts
    let cohorts = manager.get_cohorts().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve cohorts".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "data": cohorts
    })))
}
