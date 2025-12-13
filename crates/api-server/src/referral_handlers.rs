// Referral program API handlers
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;
use chrono::Utc;

use crate::{ApiServer, handlers::ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct GenerateReferralRequest {
    pub display_name: String,
    pub reward_type: String, // "percentage" or "fixed"
    pub reward_amount: i64,
    pub max_referrals: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TrackReferralRequest {
    pub code: String,
    pub email: String,
    pub utm_source: Option<String>,
    pub utm_campaign: Option<String>,
    pub utm_medium: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateReferralRequest {
    pub referral_code: String,
}

#[derive(Debug, Serialize)]
pub struct ReferralResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Generate a referral code for an authenticated user
pub async fn generate_referral_code(
    State(state): State<Arc<ApiServer>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<GenerateReferralRequest>,
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

    if let Some(referral_mgr) = state.referral_manager() {
        // Check if user already has a referral code
        match referral_mgr.get_referral_code(api_key).await {
            Ok(Some(_)) => {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "AlreadyExists".to_string(),
                        message: "You already have a referral code".to_string(),
                    }),
                ));
            }
            Ok(None) => {}
            Err(e) => {
                error!("Failed to check referral code: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "CheckFailed".to_string(),
                        message: "Failed to check existing code".to_string(),
                    }),
                ));
            }
        }

        // Validate reward type
        if req.reward_type != "percentage" && req.reward_type != "fixed" {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidType".to_string(),
                    message: "Reward type must be 'percentage' or 'fixed'".to_string(),
                }),
            ));
        }

        // Validate amount
        if (req.reward_type == "percentage" && (req.reward_amount < 1 || req.reward_amount > 100))
            || (req.reward_type == "fixed" && req.reward_amount < 1)
        {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "InvalidAmount".to_string(),
                    message: "Invalid reward amount for type".to_string(),
                }),
            ));
        }

        let reward_desc = match req.reward_type.as_str() {
            "percentage" => format!("{}% off", req.reward_amount),
            "fixed" => format!("${:.2} off", req.reward_amount as f64 / 100.0),
            _ => "Unknown reward".to_string(),
        };

        match referral_mgr
            .generate_referral_code(
                api_key,
                &req.display_name,
                &req.reward_type,
                req.reward_amount,
                &reward_desc,
                req.max_referrals,
            )
            .await
        {
            Ok(referral) => {
                Ok(Json(json!({
                    "success": true,
                    "referral": {
                        "id": referral.id,
                        "code": referral.code,
                        "display_name": referral.display_name,
                        "reward_type": referral.reward_type,
                        "reward_amount": referral.reward_amount,
                        "reward_description": referral.reward_description,
                        "max_referrals": referral.max_referrals,
                        "current_referrals": referral.current_referrals,
                        "active": referral.active,
                        "created_at": referral.created_at,
                    }
                })))
            }
            Err(e) => {
                error!("Failed to generate referral code: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "GenerationFailed".to_string(),
                        message: format!("Failed to generate code: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "ReferralUnavailable".to_string(),
                message: "Referral system not available".to_string(),
            }),
        ))
    }
}

/// Get referral code for authenticated user
pub async fn get_my_referral_code(
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

    if let Some(referral_mgr) = state.referral_manager() {
        match referral_mgr.get_referral_code(api_key).await {
            Ok(Some(code)) => {
                let stats = referral_mgr
                    .get_referral_stats(api_key)
                    .await
                    .unwrap_or_default();

                Ok(Json(json!({
                    "success": true,
                    "referral": {
                        "code": code.code,
                        "display_name": code.display_name,
                        "reward_description": code.reward_description,
                        "active": code.active,
                        "created_at": code.created_at,
                    },
                    "stats": {
                        "total_referrals": stats.total_referrals,
                        "active_referrals": stats.active_referrals,
                        "pending_referrals": stats.pending_referrals,
                        "total_rewards_earned": stats.total_rewards_earned,
                        "total_rewards_value": stats.total_rewards_value,
                    }
                })))
            }
            Ok(None) => {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "NotFound".to_string(),
                        message: "No referral code found for this user".to_string(),
                    }),
                ))
            }
            Err(e) => {
                error!("Failed to get referral code: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "FetchFailed".to_string(),
                        message: format!("Failed to fetch code: {}", e),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "ReferralUnavailable".to_string(),
                message: "Referral system not available".to_string(),
            }),
        ))
    }
}

/// Track a referral click (public endpoint)
pub async fn track_referral_click(
    State(state): State<Arc<ApiServer>>,
    Json(req): Json<TrackReferralRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(referral_mgr) = state.referral_manager() {
        // Get referral code
        let referral_code = match referral_mgr.get_referral_code_by_code(&req.code).await {
            Ok(Some(code)) => code,
            Ok(None) => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "NotFound".to_string(),
                        message: "Referral code not found".to_string(),
                    }),
                ));
            }
            Err(e) => {
                error!("Failed to get referral code: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "LookupFailed".to_string(),
                        message: "Failed to look up referral code".to_string(),
                    }),
                ));
            }
        };

        // Check if code can accept more referrals
        if !referral_mgr.can_accept_referral(referral_code.id).await.unwrap_or(false) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "LimitReached".to_string(),
                    message: "This referral code has reached its limit".to_string(),
                }),
            ));
        }

        // Build tracking data JSON
        let tracking_data = json!({
            "utm_source": req.utm_source,
            "utm_campaign": req.utm_campaign,
            "utm_medium": req.utm_medium,
            "timestamp": Utc::now().timestamp(),
        });

        match referral_mgr
            .track_referral_click(referral_code.id, &req.email, &tracking_data.to_string())
            .await
        {
            Ok(reward) => {
                Ok(Json(json!({
                    "success": true,
                    "message": "Referral tracked successfully",
                    "referrer": {
                        "name": referral_code.display_name,
                        "reward_description": referral_code.reward_description,
                    },
                    "reward": {
                        "id": reward.id,
                        "status": reward.status,
                        "reward_amount": reward.reward_amount,
                    }
                })))
            }
            Err(e) => {
                error!("Failed to track referral: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "TrackingFailed".to_string(),
                        message: "Failed to track referral".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "ReferralUnavailable".to_string(),
                message: "Referral system not available".to_string(),
            }),
        ))
    }
}

/// Get referral rewards for authenticated user
pub async fn get_my_referral_rewards(
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

    if let Some(referral_mgr) = state.referral_manager() {
        match referral_mgr.get_referral_rewards(api_key).await {
            Ok(rewards) => {
                Ok(Json(json!({
                    "success": true,
                    "rewards": rewards.iter().map(|r| json!({
                        "id": r.id,
                        "referred_email": r.referred_email,
                        "status": r.status,
                        "reward_earned": r.reward_earned,
                        "reward_amount": r.reward_amount,
                        "referred_at": r.referred_at,
                        "activated_at": r.activated_at,
                        "claimed_at": r.claimed_at,
                    })).collect::<Vec<_>>(),
                    "count": rewards.len(),
                })))
            }
            Err(e) => {
                error!("Failed to get referral rewards: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "FetchFailed".to_string(),
                        message: "Failed to fetch rewards".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "ReferralUnavailable".to_string(),
                message: "Referral system not available".to_string(),
            }),
        ))
    }
}

/// Get referral leaderboard (public)
pub async fn get_leaderboard(
    State(state): State<Arc<ApiServer>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(10);

    if let Some(referral_mgr) = state.referral_manager() {
        match referral_mgr.get_leaderboard(limit).await {
            Ok(leaders) => {
                Ok(Json(json!({
                    "success": true,
                    "leaderboard": leaders.iter().map(|l| json!({
                        "rank": l.rank,
                        "name": l.display_name,
                        "referral_code": l.referral_code,
                        "successful_referrals": l.successful_referrals,
                        "total_rewards_earned": l.total_rewards_earned,
                        "total_rewards_value": format!("${:.2}", l.total_rewards_earned as f64 / 100.0),
                    })).collect::<Vec<_>>(),
                })))
            }
            Err(e) => {
                error!("Failed to get leaderboard: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "FetchFailed".to_string(),
                        message: "Failed to fetch leaderboard".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "ReferralUnavailable".to_string(),
                message: "Referral system not available".to_string(),
            }),
        ))
    }
}

/// Get public referral details by code
pub async fn get_referral_details(
    State(state): State<Arc<ApiServer>>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(referral_mgr) = state.referral_manager() {
        match referral_mgr.get_referral_details(&code).await {
            Ok(Some((referral, stats))) => {
                Ok(Json(json!({
                    "success": true,
                    "referral": {
                        "code": referral.code,
                        "display_name": referral.display_name,
                        "reward_description": referral.reward_description,
                        "reward_type": referral.reward_type,
                        "reward_amount": referral.reward_amount,
                    },
                    "stats": {
                        "successful_referrals": stats.active_referrals,
                        "total_rewards_earned": stats.total_rewards_earned,
                        "total_rewards_value": stats.total_rewards_value,
                    }
                })))
            }
            Ok(None) => {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: "NotFound".to_string(),
                        message: "Referral code not found".to_string(),
                    }),
                ))
            }
            Err(e) => {
                error!("Failed to get referral details: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "FetchFailed".to_string(),
                        message: "Failed to fetch referral details".to_string(),
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "ReferralUnavailable".to_string(),
                message: "Referral system not available".to_string(),
            }),
        ))
    }
}
