// Annual Billing API Handlers
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::{ApiServer, handlers::ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct UpgradeToAnnualRequest {
    pub tier: String,
}

#[derive(Debug, Serialize)]
pub struct AnnualPlanResponse {
    pub success: bool,
    pub plan: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

/// Get annual billing breakdown for a tier (pricing info)
pub async fn get_annual_pricing(
    State(state): State<Arc<ApiServer>>,
    Path(tier): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(annual_mgr) = state.annual_billing_manager() {
        match annual_mgr.get_billing_breakdown(&tier).await {
            Ok(breakdown) => {
                Ok(Json(json!({
                    "success": true,
                    "breakdown": {
                        "tier": breakdown.tier,
                        "monthly_price": breakdown.monthly_price,
                        "annual_regular_price": breakdown.annual_regular_price,
                        "discount_percentage": breakdown.discount_percentage,
                        "discount_amount": breakdown.discount_amount,
                        "annual_discounted_price": breakdown.annual_discounted_price,
                        "monthly_effective_price": breakdown.monthly_effective_price,
                        "savings_per_month": breakdown.savings_per_month,
                        "total_annual_savings": breakdown.total_annual_savings,
                    }
                })))
            }
            Err(e) => {
                error!("Failed to get annual pricing: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "Failed to get annual pricing".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service_unavailable".to_string(),
                message: "Annual billing service not available".to_string(),
            }),
        ))
    }
}

/// Upgrade to annual billing
pub async fn upgrade_to_annual(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<UpgradeToAnnualRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(annual_mgr) = state.annual_billing_manager() {
        // Validate tier
        if !["trial", "pro", "enterprise"].contains(&req.tier.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_tier".to_string(),
                    message: "Invalid tier. Must be 'trial', 'pro', or 'enterprise'".to_string(),
                }),
            ));
        }

        // Check if user already has annual plan
        match annual_mgr.get_annual_plan(&api_key).await {
            Ok(Some(_)) => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "already_annual".to_string(),
                        message: "User already has an active annual plan".to_string(),
                    }),
                ))
            }
            Ok(None) => {},
            Err(e) => {
                error!("Failed to check annual plan: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "Failed to check existing plan".to_string(),
                    }),
                ));
            }
        }

        match annual_mgr.upgrade_to_annual(&api_key, &req.tier).await {
            Ok(plan) => {
                Ok(Json(json!({
                    "success": true,
                    "plan": {
                        "id": plan.id,
                        "tier": plan.tier,
                        "annual_price": plan.annual_price,
                        "monthly_equivalent": plan.monthly_equivalent,
                        "discount_percentage": plan.discount_percentage,
                        "billing_start_date": plan.billing_start_date,
                        "billing_end_date": plan.billing_end_date,
                        "auto_renew": plan.auto_renew,
                        "status": plan.status,
                    }
                })))
            }
            Err(e) => {
                error!("Failed to upgrade to annual: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "upgrade_failed".to_string(),
                        message: "Failed to upgrade to annual billing".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service_unavailable".to_string(),
                message: "Annual billing service not available".to_string(),
            }),
        ))
    }
}

/// Get current annual plan status
pub async fn get_annual_plan(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(annual_mgr) = state.annual_billing_manager() {
        match annual_mgr.get_annual_plan(&api_key).await {
            Ok(Some(plan)) => {
                Ok(Json(json!({
                    "success": true,
                    "plan": {
                        "id": plan.id,
                        "tier": plan.tier,
                        "annual_price": plan.annual_price,
                        "monthly_equivalent": plan.monthly_equivalent,
                        "discount_percentage": plan.discount_percentage,
                        "billing_start_date": plan.billing_start_date,
                        "billing_end_date": plan.billing_end_date,
                        "auto_renew": plan.auto_renew,
                        "status": plan.status,
                    }
                })))
            }
            Ok(None) => {
                Ok(Json(json!({
                    "success": true,
                    "plan": null,
                    "message": "No active annual plan"
                })))
            }
            Err(e) => {
                error!("Failed to get annual plan: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "Failed to retrieve annual plan".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service_unavailable".to_string(),
                message: "Annual billing service not available".to_string(),
            }),
        ))
    }
}

/// Get annual subscription usage and renewal info
pub async fn get_annual_subscription_usage(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(annual_mgr) = state.annual_billing_manager() {
        match annual_mgr.get_annual_subscription_usage(&api_key).await {
            Ok(Some(usage)) => {
                Ok(Json(json!({
                    "success": true,
                    "usage": {
                        "tier": usage.tier,
                        "billing_period": usage.billing_period,
                        "days_remaining": usage.days_remaining,
                        "percentage_used": format!("{:.1}", usage.percentage_used),
                        "renewal_date": usage.renewal_date,
                        "next_charge_amount": usage.next_charge_amount,
                        "can_downgrade": usage.can_downgrade,
                    }
                })))
            }
            Ok(None) => {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "no_annual_plan".to_string(),
                        message: "User does not have an active annual plan".to_string(),
                    }),
                ))
            }
            Err(e) => {
                error!("Failed to get annual subscription usage: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "Failed to retrieve usage information".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service_unavailable".to_string(),
                message: "Annual billing service not available".to_string(),
            }),
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct DowngradeRequest {
    pub reason: Option<String>,
}

/// Downgrade from annual to monthly billing
pub async fn downgrade_from_annual(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<DowngradeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(annual_mgr) = state.annual_billing_manager() {
        let reason = req.reason.unwrap_or_else(|| "User initiated downgrade".to_string());

        match annual_mgr.downgrade_from_annual(&api_key, &reason).await {
            Ok(true) => {
                Ok(Json(json!({
                    "success": true,
                    "message": "Successfully downgraded to monthly billing",
                    "effective_immediately": true
                })))
            }
            Ok(false) => {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "no_annual_plan".to_string(),
                        message: "No active annual plan found to downgrade".to_string(),
                    }),
                ))
            }
            Err(e) => {
                error!("Failed to downgrade from annual: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "Failed to downgrade plan".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service_unavailable".to_string(),
                message: "Annual billing service not available".to_string(),
            }),
        ))
    }
}

/// Get annual billing statistics (admin endpoint)
pub async fn get_annual_billing_stats(
    State(state): State<Arc<ApiServer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(annual_mgr) = state.annual_billing_manager() {
        match annual_mgr.get_annual_billing_stats().await {
            Ok(stats) => {
                Ok(Json(json!({
                    "success": true,
                    "stats": {
                        "total_annual_subscribers": stats.total_annual_subscribers,
                        "total_annual_revenue": stats.total_annual_revenue,
                        "mrr_equivalent": stats.mrr_equivalent,
                        "average_discount_taken": format!("{:.1}", stats.average_discount_taken),
                        "retention_rate": format!("{:.1}", stats.retention_rate),
                        "churn_rate": format!("{:.1}", stats.churn_rate),
                    }
                })))
            }
            Err(e) => {
                error!("Failed to get annual billing stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "internal_error".to_string(),
                        message: "Failed to retrieve billing statistics".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "service_unavailable".to_string(),
                message: "Annual billing service not available".to_string(),
            }),
        ))
    }
}
