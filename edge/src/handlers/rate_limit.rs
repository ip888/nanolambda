//! Rate limiter Durable Object handler

use worker::{Request, Response};
use crate::RateLimiter;

/// Handle requests to the RateLimiter Durable Object
pub async fn handle_request(limiter: &RateLimiter, mut req: Request) -> worker::Result<Response> {
    let url = req.url()?;
    let path = url.path();
    
    match path {
        "/check" => {
            // Check if request is allowed
            let body: CheckRequest = req.json().await?;
            
            let storage = limiter.state().storage();
            let now = chrono::Utc::now();
            let minute_key = format!("minute:{}", now.format("%Y%m%d%H%M"));
            let day_key = format!("day:{}", now.format("%Y%m%d"));
            
            // Get current counts
            let minute_count: u32 = storage.get(&minute_key).await?.unwrap_or(0);
            let day_count: u32 = storage.get(&day_key).await?.unwrap_or(0);
            
            // Check limits
            let allowed = minute_count < body.requests_per_minute 
                && day_count < body.requests_per_day;
            
            if allowed {
                // Increment counters
                storage.put(&minute_key, &(minute_count + 1)).await?;
                storage.put(&day_key, &(day_count + 1)).await?;
                
                // Set TTL for cleanup (minute key expires after 2 minutes, day key after 2 days)
                // Note: Workers KV doesn't support TTL on DO storage, 
                // we'd need a cleanup mechanism
            }
            
            Response::from_json(&CheckResponse {
                allowed,
                remaining_minute: body.requests_per_minute.saturating_sub(minute_count + if allowed { 1 } else { 0 }),
                remaining_day: body.requests_per_day.saturating_sub(day_count + if allowed { 1 } else { 0 }),
                reset_minute: 60 - (now.timestamp() % 60) as u32,
                reset_day: 86400 - (now.timestamp() % 86400) as u32,
            })
        }
        "/stats" => {
            // Get current rate limit stats
            let storage = limiter.state().storage();
            let now = chrono::Utc::now();
            let minute_key = format!("minute:{}", now.format("%Y%m%d%H%M"));
            let day_key = format!("day:{}", now.format("%Y%m%d"));
            
            let minute_count: u32 = storage.get(&minute_key).await?.unwrap_or(0);
            let day_count: u32 = storage.get(&day_key).await?.unwrap_or(0);
            
            Response::from_json(&serde_json::json!({
                "current_minute_requests": minute_count,
                "current_day_requests": day_count,
                "timestamp": now.to_rfc3339()
            }))
        }
        "/reset" => {
            // Reset all counters (admin only)
            let storage = limiter.state().storage();
            storage.delete_all().await?;
            
            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Rate limits reset"
            }))
        }
        _ => Response::error("Not found", 404),
    }
}

#[derive(serde::Deserialize)]
struct CheckRequest {
    requests_per_minute: u32,
    requests_per_day: u32,
}

#[derive(serde::Serialize)]
struct CheckResponse {
    allowed: bool,
    remaining_minute: u32,
    remaining_day: u32,
    reset_minute: u32,
    reset_day: u32,
}
