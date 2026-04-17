// Discount code API handlers
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

use crate::{ApiServer, handlers::ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct CreateDiscountRequest {
    pub code: String,
    pub discount_type: String, // "percentage" or "fixed"
    pub amount: i64,
    pub description: String,
    pub max_uses: Option<i64>,
    pub expires_at: Option<i64>,
    pub create_stripe_coupon: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateDiscountRequest {
    pub code: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct ApplyDiscountRequest {
    pub code: String,
    pub amount: i64,
}

/// Create a new discount code (admin only - requires ADMIN_API_KEY)
pub async fn create_discount(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateDiscountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Verify admin API key (should be stored securely in environment)
    let admin_key =
        std::env::var("ADMIN_API_KEY").unwrap_or_else(|_| "admin-key-change-me".to_string());

    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if api_key != admin_key {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Forbidden".to_string(),
                message: "Admin privileges required".to_string(),
            }),
        ));
    }

    if let Some(discount_mgr) = state.discount_manager() {
        // Parse discount type
        let discount_type = match req.discount_type.to_lowercase().as_str() {
            "percentage" => nanolambda_storage::discount::DiscountType::Percentage,
            "fixed" => nanolambda_storage::discount::DiscountType::Fixed,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "InvalidDiscountType".to_string(),
                        message: "Discount type must be 'percentage' or 'fixed'".to_string(),
                    }),
                ));
            }
        };

        match discount_mgr
            .create_discount(
                req.code,
                discount_type,
                req.amount,
                req.description,
                req.max_uses,
                req.expires_at,
                None, // Stripe coupon ID will be filled if we create one
            )
            .await
        {
            Ok(mut discount) => {
                // Optionally create Stripe coupon
                if req.create_stripe_coupon.unwrap_or(false) {
                    match discount_mgr.create_stripe_coupon(discount.id).await {
                        Ok(coupon_id) => {
                            discount.stripe_coupon_id = Some(coupon_id);
                        }
                        Err(e) => {
                            error!("Failed to create Stripe coupon: {}", e);
                            // Continue anyway - local discount still works
                        }
                    }
                }

                Ok(Json(json!({
                    "success": true,
                    "discount": {
                        "id": discount.id,
                        "code": discount.code,
                        "type": discount.discount_type.as_str(),
                        "amount": discount.amount,
                        "description": discount.description,
                        "max_uses": discount.max_uses,
                        "current_uses": discount.current_uses,
                        "expires_at": discount.expires_at,
                        "stripe_coupon_id": discount.stripe_coupon_id,
                        "active": discount.active,
                        "created_at": discount.created_at,
                    }
                })))
            }
            Err(e) => {
                error!("Failed to create discount code: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "DiscountCreationFailed".to_string(),
                        message: format!("Failed to create discount code: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "DiscountUnavailable".to_string(),
                message: "Discount system not available".to_string(),
            }),
        ))
    }
}

/// Validate a discount code (public endpoint - no auth required for validation)
pub async fn validate_discount(
    State(state): State<Arc<ApiServer>>,
    Json(req): Json<ValidateDiscountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(discount_mgr) = state.discount_manager() {
        match discount_mgr.validate_discount(&req.code, req.amount).await {
            Ok(validation) => Ok(Json(json!({
                "success": true,
                "valid": validation.valid,
                "error_message": validation.error_message,
                "calculated_discount": validation.calculated_discount,
                "discount_code": validation.discount_code.map(|d| json!({
                    "id": d.id,
                    "code": d.code,
                    "type": d.discount_type.as_str(),
                    "amount": d.amount,
                    "description": d.description,
                })),
            }))),
            Err(e) => {
                error!("Failed to validate discount code: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "ValidationFailed".to_string(),
                        message: format!("Failed to validate discount code: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "DiscountUnavailable".to_string(),
                message: "Discount system not available".to_string(),
            }),
        ))
    }
}

/// Apply a discount code (requires authentication)
pub async fn apply_discount(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ApplyDiscountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(discount_mgr) = state.discount_manager() {
        match discount_mgr
            .apply_discount(&req.code, api_key, req.amount)
            .await
        {
            Ok(usage) => Ok(Json(json!({
                "success": true,
                "usage": {
                    "id": usage.id,
                    "discount_code_id": usage.discount_code_id,
                    "applied_amount": usage.applied_amount,
                    "original_amount": usage.original_amount,
                    "final_amount": usage.final_amount,
                    "used_at": usage.used_at,
                }
            }))),
            Err(e) => {
                error!("Failed to apply discount code: {}", e);
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "DiscountApplicationFailed".to_string(),
                        message: format!("Failed to apply discount code: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "DiscountUnavailable".to_string(),
                message: "Discount system not available".to_string(),
            }),
        ))
    }
}

/// List all discount codes (admin only)
pub async fn list_discounts(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Verify admin API key
    let admin_key =
        std::env::var("ADMIN_API_KEY").unwrap_or_else(|_| "admin-key-change-me".to_string());

    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if api_key != admin_key {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Forbidden".to_string(),
                message: "Admin privileges required".to_string(),
            }),
        ));
    }

    if let Some(discount_mgr) = state.discount_manager() {
        let active_only = params
            .get("active_only")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        match discount_mgr.list_discounts(active_only).await {
            Ok(discounts) => Ok(Json(json!({
                "success": true,
                "discounts": discounts.iter().map(|d| json!({
                    "id": d.id,
                    "code": d.code,
                    "type": d.discount_type.as_str(),
                    "amount": d.amount,
                    "description": d.description,
                    "max_uses": d.max_uses,
                    "current_uses": d.current_uses,
                    "expires_at": d.expires_at,
                    "stripe_coupon_id": d.stripe_coupon_id,
                    "active": d.active,
                    "created_at": d.created_at,
                })).collect::<Vec<_>>(),
                "count": discounts.len(),
            }))),
            Err(e) => {
                error!("Failed to list discount codes: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "ListFailed".to_string(),
                        message: format!("Failed to list discount codes: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "DiscountUnavailable".to_string(),
                message: "Discount system not available".to_string(),
            }),
        ))
    }
}

/// Get discount usage history for a code (admin only)
pub async fn get_discount_usage(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Path(discount_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Verify admin API key
    let admin_key =
        std::env::var("ADMIN_API_KEY").unwrap_or_else(|_| "admin-key-change-me".to_string());

    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if api_key != admin_key {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Forbidden".to_string(),
                message: "Admin privileges required".to_string(),
            }),
        ));
    }

    if let Some(discount_mgr) = state.discount_manager() {
        match discount_mgr.get_discount_usage(discount_id).await {
            Ok(usage) => Ok(Json(json!({
                "success": true,
                "usage": usage.iter().map(|u| json!({
                    "id": u.id,
                    "discount_code_id": u.discount_code_id,
                    "api_key": u.api_key,
                    "applied_amount": u.applied_amount,
                    "original_amount": u.original_amount,
                    "final_amount": u.final_amount,
                    "used_at": u.used_at,
                })).collect::<Vec<_>>(),
                "count": usage.len(),
            }))),
            Err(e) => {
                error!("Failed to get discount usage: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "UsageFetchFailed".to_string(),
                        message: format!("Failed to fetch usage: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "DiscountUnavailable".to_string(),
                message: "Discount system not available".to_string(),
            }),
        ))
    }
}

/// Get user's own discount usage history (authenticated users)
pub async fn get_user_discount_usage(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    message: "API key required".to_string(),
                }),
            )
        })?;

    if let Some(discount_mgr) = state.discount_manager() {
        match discount_mgr.get_user_discount_usage(api_key).await {
            Ok(usage) => Ok(Json(json!({
                "success": true,
                "usage": usage.iter().map(|u| json!({
                    "id": u.id,
                    "discount_code_id": u.discount_code_id,
                    "applied_amount": u.applied_amount,
                    "original_amount": u.original_amount,
                    "final_amount": u.final_amount,
                    "used_at": u.used_at,
                })).collect::<Vec<_>>(),
                "count": usage.len(),
            }))),
            Err(e) => {
                error!("Failed to get user discount usage: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "UsageFetchFailed".to_string(),
                        message: format!("Failed to fetch usage: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "DiscountUnavailable".to_string(),
                message: "Discount system not available".to_string(),
            }),
        ))
    }
}
