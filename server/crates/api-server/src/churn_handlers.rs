// Churn Prevention API Handlers
// HTTP endpoints for churn analysis and intervention tracking

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::ApiServer;
use crate::handlers::ErrorResponse;

/// Request body for analyzing churn risk
#[derive(Debug, Deserialize)]
pub struct AnalyzeChurnRiskRequest {
    pub usage_declining: bool,
    pub payment_issues: i64,
    pub support_tickets: i64,
    pub last_login_days_ago: i64,
    pub feature_adoption_score: f64,
    pub nps_score: Option<i64>,
    pub predicted_ltv: i64,
}

/// Request body for recording churn event
#[derive(Debug, Deserialize)]
pub struct RecordChurnRequest {
    pub account_age_days: i64,
    pub total_revenue_lost: i64,
    pub churn_reason: Option<String>,
}

/// Request body for recording intervention
#[derive(Debug, Deserialize)]
pub struct RecordInterventionRequest {
    pub action: String,
    pub priority: String,
    pub estimated_cost: i64,
    pub expected_success_rate: f64,
    pub timeline: String,
    pub owner: String,
}

/// Analyze churn risk for authenticated customer
pub async fn analyze_churn_risk(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<AnalyzeChurnRiskRequest>,
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

    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Analyze churn risk
    let profile = analyzer
        .analyze_churn_risk(
            api_key_str,
            payload.usage_declining,
            payload.payment_issues,
            payload.support_tickets,
            payload.last_login_days_ago,
            payload.feature_adoption_score,
            payload.nps_score,
            payload.predicted_ltv,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Analysis failed".to_string(),
                    message: e.to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "data": profile
    })))
}

/// Get churn risk profile for authenticated customer
pub async fn get_risk_profile(
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

    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Get risk profile
    let profile = analyzer.get_risk_profile(api_key_str).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve profile".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    if let Some(profile_data) = profile {
        Ok(Json(json!({
            "success": true,
            "data": profile_data
        })))
    } else {
        Ok(Json(json!({
            "success": true,
            "data": {
                "message": "No churn analysis yet. Use POST /churn/analyze to compute.",
                "api_key": api_key_str
            }
        })))
    }
}

/// Record churn event for customer
pub async fn record_churn(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RecordChurnRequest>,
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

    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Record churn
    analyzer
        .record_churn(
            api_key_str,
            payload.account_age_days,
            payload.total_revenue_lost,
            payload.churn_reason,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to record churn".to_string(),
                    message: e.to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": "Churn event recorded successfully"
    })))
}

/// Predict churn for a time period
pub async fn predict_churn(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Generate predictions for multiple periods
    let week_prediction = analyzer.predict_churn("next-week").await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Prediction failed".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    let month_prediction = analyzer.predict_churn("next-month").await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Prediction failed".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    let quarter_prediction = analyzer.predict_churn("next-quarter").await.map_err(|e| {
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
        "data": {
            "next_week": week_prediction,
            "next_month": month_prediction,
            "next_quarter": quarter_prediction
        }
    })))
}

/// Get platform churn metrics (public - admin view)
pub async fn get_platform_metrics(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Get platform metrics
    let metrics = analyzer.get_platform_metrics().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve metrics".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "data": metrics
    })))
}

/// Record intervention taken for customer
pub async fn record_intervention(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RecordInterventionRequest>,
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

    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Create intervention
    let intervention = nanolambda_storage::churn::Intervention {
        action: payload.action,
        priority: payload.priority,
        estimated_cost: payload.estimated_cost,
        expected_success_rate: payload.expected_success_rate,
        timeline: payload.timeline,
        owner: payload.owner,
    };

    // Record intervention
    analyzer
        .record_intervention(api_key_str, intervention)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to record intervention".to_string(),
                    message: e.to_string(),
                }),
            )
        })?;

    Ok(Json(json!({
        "success": true,
        "message": "Intervention recorded successfully"
    })))
}

/// Get interventions for authenticated customer
pub async fn get_interventions(
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

    // Get churn analyzer
    let analyzer = state.churn_analyzer().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Service unavailable".to_string(),
            message: "Churn analyzer not available".to_string(),
        }),
    ))?;

    // Get interventions
    let interventions = analyzer.get_interventions(api_key_str).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to retrieve interventions".to_string(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "data": interventions
    })))
}
