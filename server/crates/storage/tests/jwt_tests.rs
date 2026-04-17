//! Integration tests for JWT token management
//!
//! Tests token generation, validation, refresh, and session management.

use nanolambda_storage::jwt::JwtManager;

/// Helper to create an in-memory JWT manager for testing
async fn setup_jwt_manager() -> JwtManager {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create the sessions table (normally created by UserManager)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            refresh_token_hash TEXT NOT NULL,
            user_agent TEXT,
            ip_address TEXT,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            revoked_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create sessions table");

    // Create minimal users table for refresh token tests (which joins with users)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT,
            plan TEXT NOT NULL DEFAULT 'free',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_login_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    JwtManager::new(pool)
        .await
        .expect("Failed to create JwtManager")
}

/// Setup JWT manager with a test user for refresh token tests
async fn setup_jwt_manager_with_user(user_id: &str, email: &str, plan: &str) -> JwtManager {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create the sessions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            refresh_token_hash TEXT NOT NULL,
            user_agent TEXT,
            ip_address TEXT,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            revoked_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create sessions table");

    // Create users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT,
            plan TEXT NOT NULL DEFAULT 'free',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_login_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    // Insert test user
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, plan, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(email)
    .bind("hashed_password")
    .bind(plan)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to create test user");

    JwtManager::new(pool)
        .await
        .expect("Failed to create JwtManager")
}

// =============================================================================
// Token Generation Tests
// =============================================================================

#[tokio::test]
async fn test_generate_tokens_success() {
    let manager = setup_jwt_manager().await;

    let tokens = manager
        .generate_tokens(
            "user-123",
            "test@example.com",
            "free",
            Some("TestBrowser/1.0"),
            Some("127.0.0.1"),
        )
        .await
        .expect("Token generation failed");

    // Access token should be non-empty JWT format
    assert!(!tokens.access_token.is_empty());
    assert!(tokens.access_token.contains('.'));
    let parts: Vec<&str> = tokens.access_token.split('.').collect();
    assert_eq!(parts.len(), 3); // header.payload.signature

    // Refresh token should be non-empty
    assert!(!tokens.refresh_token.is_empty());

    // Token type should be Bearer
    assert_eq!(tokens.token_type, "Bearer");

    // Expires in should be ~1 hour (3600 seconds)
    assert!(tokens.expires_in > 3500 && tokens.expires_in <= 3600);
}

#[tokio::test]
async fn test_generate_tokens_creates_session() {
    let manager = setup_jwt_manager().await;

    manager
        .generate_tokens(
            "user-456",
            "session@example.com",
            "pro",
            Some("Chrome/100"),
            Some("192.168.1.1"),
        )
        .await
        .expect("Token generation failed");

    // Session should be created
    let sessions = manager
        .get_sessions("user-456")
        .await
        .expect("Failed to get sessions");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].user_agent, Some("Chrome/100".to_string()));
    assert_eq!(sessions[0].ip_address, Some("192.168.1.1".to_string()));
    assert!(!sessions[0].revoked_at.is_some());
}

#[tokio::test]
async fn test_multiple_sessions_tracked() {
    let manager = setup_jwt_manager().await;

    // Generate tokens from different "devices"
    manager
        .generate_tokens(
            "user-multi",
            "multi@example.com",
            "free",
            Some("Browser1"),
            Some("10.0.0.1"),
        )
        .await
        .unwrap();
    manager
        .generate_tokens(
            "user-multi",
            "multi@example.com",
            "free",
            Some("Browser2"),
            Some("10.0.0.2"),
        )
        .await
        .unwrap();
    manager
        .generate_tokens(
            "user-multi",
            "multi@example.com",
            "free",
            Some("Mobile"),
            Some("10.0.0.3"),
        )
        .await
        .unwrap();

    let sessions = manager.get_sessions("user-multi").await.unwrap();
    assert_eq!(sessions.len(), 3);
}

// =============================================================================
// Token Validation Tests
// =============================================================================

#[tokio::test]
async fn test_validate_token_success() {
    let manager = setup_jwt_manager().await;

    let tokens = manager
        .generate_tokens("validate-user", "validate@example.com", "scale", None, None)
        .await
        .expect("Token generation failed");

    let claims = manager
        .validate_token(&tokens.access_token)
        .expect("Token validation failed");

    assert_eq!(claims.sub, "validate-user");
    assert_eq!(claims.email, "validate@example.com");
    assert_eq!(claims.plan, "scale");
}

#[tokio::test]
async fn test_validate_token_invalid_format_rejected() {
    let manager = setup_jwt_manager().await;

    // Not a valid JWT format
    let invalid_tokens = vec![
        "not-a-jwt",
        "",
        "header.payload", // Missing signature
    ];

    for token in invalid_tokens {
        let result = manager.validate_token(token);
        assert!(result.is_err(), "Token '{}' should be rejected", token);
    }
}

#[tokio::test]
async fn test_validate_token_tampered_signature_rejected() {
    let manager = setup_jwt_manager().await;

    let tokens = manager
        .generate_tokens("tamper-user", "tamper@example.com", "free", None, None)
        .await
        .unwrap();

    // Tamper with the signature
    let parts: Vec<&str> = tokens.access_token.split('.').collect();
    let tampered = format!("{}.{}.tampered_signature", parts[0], parts[1]);

    let result = manager.validate_token(&tampered);
    assert!(result.is_err());
}

// =============================================================================
// Token Refresh Tests
// =============================================================================

#[tokio::test]
async fn test_refresh_tokens_success() {
    // Need a user in the database for refresh to work (it joins with users table)
    let manager = setup_jwt_manager_with_user("refresh-user", "refresh@example.com", "pro").await;

    let original_tokens = manager
        .generate_tokens(
            "refresh-user",
            "refresh@example.com",
            "pro",
            Some("Browser"),
            Some("1.2.3.4"),
        )
        .await
        .expect("Initial token generation failed");

    // Small delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let new_tokens = manager
        .refresh_tokens(
            &original_tokens.refresh_token,
            Some("Browser"),
            Some("1.2.3.4"),
        )
        .await
        .expect("Token refresh failed");

    // New tokens should be different
    assert_ne!(new_tokens.access_token, original_tokens.access_token);
    assert_ne!(new_tokens.refresh_token, original_tokens.refresh_token);

    // New access token should be valid
    let claims = manager.validate_token(&new_tokens.access_token).unwrap();
    assert_eq!(claims.sub, "refresh-user");
}

#[tokio::test]
async fn test_refresh_tokens_invalid_token_rejected() {
    let manager = setup_jwt_manager().await;

    let result = manager
        .refresh_tokens("invalid_refresh_token", None, None)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_refresh_tokens_revoked_token_rejected() {
    let manager = setup_jwt_manager().await;

    let tokens = manager
        .generate_tokens("revoke-refresh", "revoke@example.com", "free", None, None)
        .await
        .expect("Token generation failed");

    // Revoke all sessions for user
    manager
        .revoke_session("revoke-refresh", None)
        .await
        .expect("Revoke failed");

    // Try to refresh - should fail
    let result = manager
        .refresh_tokens(&tokens.refresh_token, None, None)
        .await;
    assert!(result.is_err());
}

// =============================================================================
// Session Management Tests
// =============================================================================

#[tokio::test]
async fn test_revoke_specific_session() {
    let manager = setup_jwt_manager().await;

    // Create multiple sessions
    manager
        .generate_tokens(
            "revoke-one",
            "revokeone@example.com",
            "free",
            Some("A"),
            None,
        )
        .await
        .unwrap();
    let tokens_b = manager
        .generate_tokens(
            "revoke-one",
            "revokeone@example.com",
            "free",
            Some("B"),
            None,
        )
        .await
        .unwrap();
    manager
        .generate_tokens(
            "revoke-one",
            "revokeone@example.com",
            "free",
            Some("C"),
            None,
        )
        .await
        .unwrap();

    let sessions_before = manager.get_sessions("revoke-one").await.unwrap();
    assert_eq!(sessions_before.len(), 3);

    // Get session ID for B (find by user_agent)
    let session_b = sessions_before
        .iter()
        .find(|s| s.user_agent == Some("B".to_string()))
        .expect("Session B not found");

    // Revoke session B
    manager
        .revoke_session("revoke-one", Some(&session_b.id))
        .await
        .expect("Revoke failed");

    // Check that only B is revoked
    let sessions_after = manager.get_sessions("revoke-one").await.unwrap();
    let active_sessions: Vec<_> = sessions_after
        .iter()
        .filter(|s| s.revoked_at.is_none())
        .collect();
    assert_eq!(active_sessions.len(), 2);

    // Token B refresh should fail
    let refresh_result = manager
        .refresh_tokens(&tokens_b.refresh_token, None, None)
        .await;
    assert!(refresh_result.is_err());
}

#[tokio::test]
async fn test_revoke_all_sessions() {
    let manager = setup_jwt_manager().await;

    // Create multiple sessions
    let tokens1 = manager
        .generate_tokens(
            "revoke-all",
            "revokeall@example.com",
            "pro",
            Some("A"),
            None,
        )
        .await
        .unwrap();
    let tokens2 = manager
        .generate_tokens(
            "revoke-all",
            "revokeall@example.com",
            "pro",
            Some("B"),
            None,
        )
        .await
        .unwrap();
    let tokens3 = manager
        .generate_tokens(
            "revoke-all",
            "revokeall@example.com",
            "pro",
            Some("C"),
            None,
        )
        .await
        .unwrap();

    // Revoke all
    manager
        .revoke_session("revoke-all", None)
        .await
        .expect("Revoke all failed");

    // All sessions should be revoked
    let sessions = manager.get_sessions("revoke-all").await.unwrap();
    let active: Vec<_> = sessions.iter().filter(|s| s.revoked_at.is_none()).collect();
    assert_eq!(active.len(), 0);

    // All refresh tokens should fail
    for tokens in [tokens1, tokens2, tokens3] {
        let result = manager
            .refresh_tokens(&tokens.refresh_token, None, None)
            .await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_cleanup_expired_sessions() {
    let manager = setup_jwt_manager().await;

    // Create a session
    manager
        .generate_tokens("cleanup-user", "cleanup@example.com", "free", None, None)
        .await
        .unwrap();

    // Cleanup (should not remove valid sessions)
    let removed = manager
        .cleanup_expired_sessions()
        .await
        .expect("Cleanup failed");

    // No sessions should be removed (they're all fresh)
    assert_eq!(removed, 0);
}

// =============================================================================
// Claims Tests
// =============================================================================

#[tokio::test]
async fn test_claims_contain_user_info() {
    let manager = setup_jwt_manager().await;

    let tokens = manager
        .generate_tokens(
            "claims-user",
            "claims@example.com",
            "enterprise",
            None,
            None,
        )
        .await
        .unwrap();
    let claims = manager.validate_token(&tokens.access_token).unwrap();

    assert_eq!(claims.sub, "claims-user");
    assert_eq!(claims.email, "claims@example.com");
    assert_eq!(claims.plan, "enterprise");
    assert!(!claims.jti.is_empty()); // JWT ID should be set
    assert!(claims.iat > 0); // Issued at should be set
    assert!(claims.exp > claims.iat); // Expiry should be after issued
}
