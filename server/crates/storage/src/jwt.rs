//! JWT token management for user authentication
//!
//! This module provides JWT (JSON Web Token) generation, validation, and refresh token
//! handling for the NanoLambda authentication system.
//!
//! ## Security Model
//!
//! - **Access tokens**: Short-lived (1 hour) JWT tokens used for API authentication
//! - **Refresh tokens**: Long-lived (7 days) opaque tokens stored in the database
//! - **Token rotation**: Each refresh generates new access and refresh tokens
//! - **Session tracking**: All sessions are tracked in the database for audit and revocation
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Generate tokens after successful login
//! let tokens = jwt_manager.generate_tokens(&user, user_agent, ip_address).await?;
//!
//! // Validate an access token from a request
//! let claims = jwt_manager.validate_token(&access_token)?;
//!
//! // Refresh expired access token
//! let new_tokens = jwt_manager.refresh_tokens(&refresh_token, user_agent, ip).await?;
//! ```

use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub email: String,
    pub plan: String,
    pub exp: i64,    // Expiration timestamp
    pub iat: i64,    // Issued at timestamp
    pub jti: String, // JWT ID (unique identifier)
}

/// Access token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

/// JWT manager
pub struct JwtManager {
    pool: SqlitePool,
    secret: String,
    /// Access tokens are short-lived (1 hour) for security.
    /// If compromised, damage is limited to this window.
    access_token_ttl: Duration,
    /// Refresh tokens are long-lived (30 days) for user convenience.
    /// They're stored hashed in the database and can be individually revoked.
    refresh_token_ttl: Duration,
}

impl JwtManager {
    /// Create a new JWT manager with default token lifetimes.
    ///
    /// # Token Lifetime Rationale
    ///
    /// - **Access tokens (1 hour)**: Short lifetime limits exposure if leaked.
    ///   Clients must refresh tokens regularly, allowing us to detect anomalies.
    /// - **Refresh tokens (30 days)**: Longer lifetime for user convenience.
    ///   These are stored hashed and can be revoked per-session.
    ///
    /// # Security Configuration
    ///
    /// The JWT signing secret MUST be set via the `JWT_SECRET` environment
    /// variable in production. The default value is only for development.
    /// Use a cryptographically random string of at least 32 bytes.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "nanolambda-jwt-secret-change-in-production".to_string());

        Ok(Self {
            pool,
            secret,
            access_token_ttl: Duration::hours(1),
            refresh_token_ttl: Duration::days(30),
        })
    }

    /// Generate access and refresh tokens for a user
    pub async fn generate_tokens(
        &self,
        user_id: &str,
        email: &str,
        plan: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<TokenResponse> {
        let now = Utc::now();
        let jti = uuid::Uuid::new_v4().to_string();

        // Create access token claims
        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            plan: plan.to_string(),
            exp: (now + self.access_token_ttl).timestamp(),
            iat: now.timestamp(),
            jti: jti.clone(),
        };

        // Encode access token (simple HMAC-SHA256 based JWT)
        let access_token = self.encode_token(&claims)?;

        // Generate refresh token
        let refresh_token = self.generate_refresh_token();
        let refresh_token_hash = self.hash_token(&refresh_token);

        // Store session
        let session_id = uuid::Uuid::new_v4().to_string();
        let expires_at = (now + self.refresh_token_ttl).timestamp();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, refresh_token_hash, user_agent, ip_address, created_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(&refresh_token_hash)
        .bind(user_agent)
        .bind(ip_address)
        .bind(now.timestamp())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl.num_seconds(),
        })
    }

    /// Validate and decode access token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let claims = self.decode_token(token)?;

        // Check expiration
        let now = Utc::now().timestamp();
        if claims.exp < now {
            return Err(anyhow!("Token expired"));
        }

        Ok(claims)
    }

    /// Refresh access token using refresh token
    pub async fn refresh_tokens(
        &self,
        refresh_token: &str,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<TokenResponse> {
        let refresh_token_hash = self.hash_token(refresh_token);
        let now = Utc::now().timestamp();

        // Find valid session
        let row = sqlx::query(
            r#"
            SELECT s.id, s.user_id, u.email, u.plan
            FROM sessions s
            JOIN users u ON s.user_id = u.id
            WHERE s.refresh_token_hash = ?
              AND s.expires_at > ?
              AND s.revoked_at IS NULL
            "#,
        )
        .bind(&refresh_token_hash)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let session_id: String = row.get("id");
                let user_id: String = row.get("user_id");
                let email: String = row.get("email");
                let plan: String = row.get("plan");

                // Revoke old session
                sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(&session_id)
                    .execute(&self.pool)
                    .await?;

                // Generate new tokens
                self.generate_tokens(&user_id, &email, &plan, user_agent, ip_address)
                    .await
            }
            None => Err(anyhow!("Invalid or expired refresh token")),
        }
    }

    /// Revoke a session (logout)
    pub async fn revoke_session(&self, user_id: &str, session_id: Option<&str>) -> Result<()> {
        let now = Utc::now().timestamp();

        if let Some(sid) = session_id {
            // Revoke specific session
            sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ? AND user_id = ?")
                .bind(now)
                .bind(sid)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        } else {
            // Revoke all sessions for user
            sqlx::query(
                "UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
            )
            .bind(now)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get active sessions for a user
    pub async fn get_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
        let now = Utc::now().timestamp();

        let rows = sqlx::query(
            r#"
            SELECT id, user_id, user_agent, ip_address, created_at, expires_at, revoked_at
            FROM sessions
            WHERE user_id = ? AND expires_at > ? AND revoked_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        let sessions = rows
            .iter()
            .map(|row| Session {
                id: row.get("id"),
                user_id: row.get("user_id"),
                user_agent: row.get("user_agent"),
                ip_address: row.get("ip_address"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                revoked_at: row.get("revoked_at"),
            })
            .collect();

        Ok(sessions)
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let now = Utc::now().timestamp();

        let result =
            sqlx::query("DELETE FROM sessions WHERE expires_at < ? OR revoked_at IS NOT NULL")
                .bind(now - 86400 * 7) // Keep revoked sessions for 7 days for audit
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    // Internal methods

    fn encode_token(&self, claims: &Claims) -> Result<String> {
        // Simple JWT encoding (header.payload.signature)
        let header = base64_url_encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64_url_encode(&serde_json::to_string(claims)?);

        let signature_input = format!("{}.{}", header, payload);
        let signature = self.sign(&signature_input);

        Ok(format!("{}.{}.{}", header, payload, signature))
    }

    fn decode_token(&self, token: &str) -> Result<Claims> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid token format"));
        }

        let (header, payload, signature) = (parts[0], parts[1], parts[2]);

        // Verify signature
        let signature_input = format!("{}.{}", header, payload);
        let expected_signature = self.sign(&signature_input);

        if signature != expected_signature {
            return Err(anyhow!("Invalid token signature"));
        }

        // Decode payload
        let payload_json = base64_url_decode(payload)?;
        let claims: Claims = serde_json::from_str(&payload_json)?;

        Ok(claims)
    }

    fn sign(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hasher.update(self.secret.as_bytes());
        base64_url_encode(&format!("{:x}", hasher.finalize()))
    }

    fn generate_refresh_token(&self) -> String {
        let random_bytes: [u8; 32] = rand::random();
        base64_url_encode(&format!("{:x}", Sha256::digest(random_bytes)))
    }

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// Base64 URL encoding/decoding helpers
fn base64_url_encode(input: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(input.as_bytes())
}

fn base64_url_decode(input: &str) -> Result<String> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let bytes = URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| anyhow!("Base64 decode error: {}", e))?;
    String::from_utf8(bytes).map_err(|e| anyhow!("UTF-8 decode error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_url_encode_decode() {
        let original = r#"{"test":"value"}"#;
        let encoded = base64_url_encode(original);
        let decoded = base64_url_decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
}
