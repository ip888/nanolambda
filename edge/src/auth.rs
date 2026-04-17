//! Authentication middleware for NanoLambda Edge

use sha2::{Sha256, Digest};
use worker::{Request, Env};
use crate::error::EdgeError;
use crate::types::{ApiKey, AuthContext, Permission, RateLimit};

/// Authenticate a request using API key or JWT
pub async fn authenticate(req: &Request, env: &Env) -> Result<AuthContext, EdgeError> {
    // Check for dev mode (local development bypass)
    let environment = env.var("ENVIRONMENT")
        .map_or_else(|_| "production".to_string(), |v| v.to_string());
    
    // Try API key first (preferred for edge)
    if let Some(api_key) = extract_api_key(req) {
        // In development, allow special dev keys
        if environment == "development" && api_key.starts_with("dev-") {
            return Ok(create_dev_context());
        }
        return authenticate_api_key(&api_key, env).await;
    }
    
    // Try JWT from Authorization header
    if let Some(token) = extract_bearer_token(req) {
        return authenticate_jwt(&token, env).await;
    }
    
    Err(EdgeError::AuthenticationFailed(
        "No valid authentication provided. Use X-Api-Key header or Bearer token.".to_string()
    ))
}

/// Create a development context with full permissions
fn create_dev_context() -> AuthContext {
    AuthContext {
        user_id: "dev-user".to_string(),
        permissions: vec![
            Permission::FunctionsRead,
            Permission::FunctionsWrite,
            Permission::FunctionsInvoke,
            Permission::VectorsRead,
            Permission::VectorsWrite,
            Permission::AiAccess,
        ],
        rate_limit: RateLimit {
            requests_per_minute: 1000,
            requests_per_day: 100_000,
            concurrent_invocations: 100,
        },
    }
}

/// Extract API key from X-Api-Key header
fn extract_api_key(req: &Request) -> Option<String> {
    req.headers()
        .get("X-Api-Key")
        .ok()
        .flatten()
}

/// Extract Bearer token from Authorization header
fn extract_bearer_token(req: &Request) -> Option<String> {
    req.headers()
        .get("Authorization")
        .ok()
        .flatten()
        .and_then(|auth| {
            auth.strip_prefix("Bearer ").map(std::string::ToString::to_string)
        })
}

/// Authenticate using API key
async fn authenticate_api_key(api_key: &str, env: &Env) -> Result<AuthContext, EdgeError> {
    // Hash the API key for lookup
    let key_hash = hash_api_key(api_key);
    
    // Look up in KV store
    let kv = env.kv("API_KEYS")
        .map_err(|e| EdgeError::Internal(format!("KV binding error: {e}")))?;
    
    let api_key_data: Option<ApiKey> = kv.get(&key_hash)
        .json()
        .await
        .map_err(|e| EdgeError::StorageError(format!("Failed to fetch API key: {e}")))?;
    
    match api_key_data {
        Some(key) => {
            // Check if key is active
            if !key.is_active {
                return Err(EdgeError::AuthorizationDenied("API key is disabled".to_string()));
            }
            
            // Check expiration
            if let Some(expires_at) = key.expires_at {
                if expires_at < chrono::Utc::now() {
                    return Err(EdgeError::AuthorizationDenied("API key has expired".to_string()));
                }
            }
            
            Ok(AuthContext {
                user_id: key.owner_id,
                permissions: key.permissions,
                rate_limit: key.rate_limit,
            })
        }
        None => Err(EdgeError::InvalidApiKey),
    }
}

/// Authenticate using JWT token
async fn authenticate_jwt(token: &str, env: &Env) -> Result<AuthContext, EdgeError> {
    // Get JWT secret from environment
    let secret = env.secret("JWT_SECRET")
        .map_err(|_| EdgeError::Internal("JWT_SECRET not configured".to_string()))?
        .to_string();
    
    // Parse and validate JWT (simplified - production should use a proper JWT library)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(EdgeError::InvalidJwt("Invalid token format".to_string()));
    }
    
    // Decode payload (base64url)
    let payload = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        parts[1]
    ).map_err(|_| EdgeError::InvalidJwt("Invalid payload encoding".to_string()))?;
    
    let claims: JwtClaims = serde_json::from_slice(&payload)
        .map_err(|e| EdgeError::InvalidJwt(format!("Invalid claims: {e}")))?;
    
    // Verify signature
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected_signature = compute_hmac_sha256(&signing_input, &secret);
    
    let provided_signature = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        parts[2]
    ).map_err(|_| EdgeError::InvalidJwt("Invalid signature encoding".to_string()))?;
    
    if expected_signature != provided_signature {
        return Err(EdgeError::InvalidJwt("Invalid signature".to_string()));
    }
    
    // Check expiration
    if let Some(exp) = claims.exp {
        let now = chrono::Utc::now().timestamp() as u64;
        if exp < now {
            return Err(EdgeError::InvalidJwt("Token has expired".to_string()));
        }
    }
    
    Ok(AuthContext {
        user_id: claims.sub,
        permissions: claims.permissions.unwrap_or_else(default_permissions),
        rate_limit: claims.rate_limit.unwrap_or_default(),
    })
}

/// Hash API key for storage lookup
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute HMAC-SHA256 for JWT signature verification
fn compute_hmac_sha256(data: &str, secret: &str) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;
    
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// Default permissions for JWT without explicit permissions
fn default_permissions() -> Vec<Permission> {
    vec![
        Permission::FunctionsRead,
        Permission::FunctionsInvoke,
        Permission::VectorsRead,
        Permission::VectorsWrite,
    ]
}

/// JWT claims structure
#[derive(serde::Deserialize)]
struct JwtClaims {
    sub: String,
    exp: Option<u64>,
    permissions: Option<Vec<Permission>>,
    rate_limit: Option<RateLimit>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_api_key() {
        let key = "nlk_test_123456789";
        let hash = hash_api_key(key);
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
    }
}
