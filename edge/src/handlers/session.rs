//! User session Durable Object handler

use crate::UserSession;
use worker::{Request, Response};

/// Handle requests to the UserSession Durable Object
pub async fn handle_request(session: &UserSession, mut req: Request) -> worker::Result<Response> {
    let url = req.url()?;
    let path = url.path();

    match path {
        "/get" => {
            // Get session data
            let storage = session.state().storage();
            let data: Option<serde_json::Value> = storage.get("session_data").await?;

            match data {
                Some(d) => Response::from_json(&d),
                None => Response::from_json(&serde_json::json!({})),
            }
        }
        "/set" => {
            // Set session data
            let body: serde_json::Value = req.json().await?;
            let storage = session.state().storage();
            storage.put("session_data", &body).await?;
            storage
                .put("last_updated", &chrono::Utc::now().to_rfc3339())
                .await?;

            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Session data updated"
            }))
        }
        "/delete" => {
            // Delete session
            let storage = session.state().storage();
            storage.delete_all().await?;

            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Session deleted"
            }))
        }
        "/touch" => {
            // Update last access time
            let storage = session.state().storage();
            storage
                .put("last_accessed", &chrono::Utc::now().to_rfc3339())
                .await?;

            Response::from_json(&serde_json::json!({
                "success": true
            }))
        }
        _ => Response::error("Not found", 404),
    }
}
