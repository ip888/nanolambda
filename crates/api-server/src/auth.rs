use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use nanolambda_storage::{ApiKeyStatus, StorageManager};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthContext {
    pub api_key_id: i64,
    pub permissions: Vec<String>,
}

/// Extract API key from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    // Format: "Bearer nl_<hash>"
    if let Some(token) = auth_str.strip_prefix("Bearer ") {
        Some(token.to_string())
    } else {
        None
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    State(storage): State<Arc<StorageManager>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let headers = req.headers();
    
    // Extract token from header
    let token = extract_bearer_token(headers)
        .ok_or(AuthError::MissingToken)?;
    
    // Validate token
    let api_key = storage
        .get_api_key(&token)
        .map_err(|_| AuthError::InvalidToken)?
        .ok_or(AuthError::InvalidToken)?;
    
    // Check if key is active
    if api_key.status != ApiKeyStatus::Active {
        return Err(AuthError::RevokedToken);
    }
    
    // Check if key is expired
    if let Some(expires_at) = api_key.expires_at {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        if now > expires_at {
            return Err(AuthError::ExpiredToken);
        }
    }
    
    // Update last used timestamp (non-blocking)
    let storage_clone = storage.clone();
    let token_clone = token.clone();
    tokio::spawn(async move {
        let _ = storage_clone.update_api_key_last_used(&token_clone);
    });
    
    // Add auth context to request extensions
    let auth_context = AuthContext {
        api_key_id: api_key.id,
        permissions: api_key.permissions,
    };
    req.extensions_mut().insert(auth_context);
    
    Ok(next.run(req).await)
}

/// Authentication errors
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    RevokedToken,
    ExpiredToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "Missing authorization token. Include 'Authorization: Bearer <token>' header.",
            ),
            AuthError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "Invalid or unknown API key",
            ),
            AuthError::RevokedToken => (
                StatusCode::FORBIDDEN,
                "API key has been revoked",
            ),
            AuthError::ExpiredToken => (
                StatusCode::FORBIDDEN,
                "API key has expired",
            ),
        };
        
        let body = json!({
            "error": message,
            "status": status.as_u16(),
        });
        
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    
    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer nl_abc123"),
        );
        
        let token = extract_bearer_token(&headers);
        assert_eq!(token, Some("nl_abc123".to_string()));
    }
    
    #[test]
    fn test_extract_missing_bearer() {
        let headers = HeaderMap::new();
        let token = extract_bearer_token(&headers);
        assert!(token.is_none());
    }
    
    #[test]
    fn test_extract_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Basic abc123"),
        );
        
        let token = extract_bearer_token(&headers);
        assert!(token.is_none());
    }
}
