//! Error types for NanoLambda Edge

use thiserror::Error;
use worker::Response;

/// Edge platform error types
#[derive(Error, Debug)]
pub enum EdgeError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Invalid JWT token: {0}")]
    InvalidJwt(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Function execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Function timeout after {0}ms")]
    Timeout(u64),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Vector operation failed: {0}")]
    VectorError(String),
    
    #[error("AI operation failed: {0}")]
    AiError(String),
    
    #[error("JavaScript runtime error: {0}")]
    JsRuntimeError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl EdgeError {
    /// Convert error to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            EdgeError::AuthenticationFailed(_) => 401,
            EdgeError::AuthorizationDenied(_) => 403,
            EdgeError::InvalidApiKey => 401,
            EdgeError::InvalidJwt(_) => 401,
            EdgeError::RateLimitExceeded(_) => 429,
            EdgeError::FunctionNotFound(_) => 404,
            EdgeError::NotFound(_) => 404,
            EdgeError::ExecutionFailed(_) => 500,
            EdgeError::Timeout(_) => 504,
            EdgeError::InvalidRequest(_) => 400,
            EdgeError::VectorError(_) => 500,
            EdgeError::AiError(_) => 500,
            EdgeError::JsRuntimeError(_) => 500,
            EdgeError::StorageError(_) => 500,
            EdgeError::Internal(_) => 500,
            EdgeError::NotImplemented(_) => 501,
        }
    }
    
    /// Convert error to JSON response
    /// 
    /// Tech Decision: We use expect() here because Response::error with a valid
    /// status code (500) is guaranteed to succeed. This is a last-resort fallback.
    pub fn to_response(&self) -> Response {
        let status = self.status_code();
        let body = serde_json::json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
            }
        });
        
        Response::from_json(&body)
            .map(|r| r.with_status(status))
            .unwrap_or_else(|_| {
                // Fallback: if JSON serialization fails, return plain text error
                // Response::error with status 500 is infallible for valid inputs
                Response::error("Internal error", 500)
                    .expect("Response::error(500) is infallible")
            })
    }
    
    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            EdgeError::AuthenticationFailed(_) => "AUTHENTICATION_FAILED",
            EdgeError::AuthorizationDenied(_) => "AUTHORIZATION_DENIED",
            EdgeError::InvalidApiKey => "INVALID_API_KEY",
            EdgeError::InvalidJwt(_) => "INVALID_JWT",
            EdgeError::RateLimitExceeded(_) => "RATE_LIMIT_EXCEEDED",
            EdgeError::FunctionNotFound(_) => "FUNCTION_NOT_FOUND",
            EdgeError::NotFound(_) => "NOT_FOUND",
            EdgeError::ExecutionFailed(_) => "EXECUTION_FAILED",
            EdgeError::Timeout(_) => "TIMEOUT",
            EdgeError::InvalidRequest(_) => "INVALID_REQUEST",
            EdgeError::VectorError(_) => "VECTOR_ERROR",
            EdgeError::AiError(_) => "AI_ERROR",
            EdgeError::JsRuntimeError(_) => "JS_RUNTIME_ERROR",
            EdgeError::StorageError(_) => "STORAGE_ERROR",
            EdgeError::Internal(_) => "INTERNAL_ERROR",
            EdgeError::NotImplemented(_) => "NOT_IMPLEMENTED",
        }
    }
}

impl From<worker::Error> for EdgeError {
    fn from(e: worker::Error) -> Self {
        EdgeError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for EdgeError {
    fn from(e: serde_json::Error) -> Self {
        EdgeError::InvalidRequest(format!("JSON parse error: {e}"))
    }
}
