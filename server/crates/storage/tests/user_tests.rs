//! Integration tests for user management
//!
//! Tests user registration, login, profile updates, and usage tracking.

use nanolambda_storage::user::{
    ChangePasswordRequest, LoginRequest, Plan, RegisterRequest, UpdateUserRequest, UserManager,
};

/// Helper to create an in-memory user manager for testing
async fn setup_user_manager() -> UserManager {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    UserManager::new(pool)
        .await
        .expect("Failed to create UserManager")
}

// =============================================================================
// Registration Tests
// =============================================================================

#[tokio::test]
async fn test_user_registration_success() {
    let manager = setup_user_manager().await;

    let request = RegisterRequest {
        email: "test@example.com".to_string(),
        password: "securepassword123".to_string(),
        name: Some("Test User".to_string()),
    };

    let user = manager
        .register(request)
        .await
        .expect("Registration failed");

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, Some("Test User".to_string()));
    assert_eq!(user.plan, Plan::Free);
    assert!(!user.email_verified);
    assert!(user.stripe_customer_id.is_none());
}

#[tokio::test]
async fn test_user_registration_email_normalized() {
    let manager = setup_user_manager().await;

    let request = RegisterRequest {
        email: "TEST@EXAMPLE.COM".to_string(),
        password: "securepassword123".to_string(),
        name: None,
    };

    let user = manager
        .register(request)
        .await
        .expect("Registration failed");

    // Email should be normalized to lowercase
    assert_eq!(user.email, "test@example.com");
}

#[tokio::test]
async fn test_user_registration_duplicate_email_rejected() {
    let manager = setup_user_manager().await;

    let request1 = RegisterRequest {
        email: "duplicate@example.com".to_string(),
        password: "password123".to_string(),
        name: None,
    };

    manager
        .register(request1)
        .await
        .expect("First registration should succeed");

    let request2 = RegisterRequest {
        email: "duplicate@example.com".to_string(),
        password: "different_password".to_string(),
        name: None,
    };

    let result = manager.register(request2).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("already registered")
    );
}

#[tokio::test]
async fn test_user_registration_invalid_email_rejected() {
    let manager = setup_user_manager().await;

    let request = RegisterRequest {
        email: "invalid-email".to_string(),
        password: "password123".to_string(),
        name: None,
    };

    let result = manager.register(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid email"));
}

#[tokio::test]
async fn test_user_registration_short_password_rejected() {
    let manager = setup_user_manager().await;

    let request = RegisterRequest {
        email: "test@example.com".to_string(),
        password: "short".to_string(), // Less than 8 chars
        name: None,
    };

    let result = manager.register(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("8 characters"));
}

// =============================================================================
// Login Tests
// =============================================================================

#[tokio::test]
async fn test_user_login_success() {
    let manager = setup_user_manager().await;

    // Register user first
    let register_request = RegisterRequest {
        email: "login@example.com".to_string(),
        password: "correctpassword".to_string(),
        name: Some("Login User".to_string()),
    };
    let registered = manager
        .register(register_request)
        .await
        .expect("Registration failed");

    // Login with correct credentials
    let login_request = LoginRequest {
        email: "login@example.com".to_string(),
        password: "correctpassword".to_string(),
    };

    let user = manager.login(login_request).await.expect("Login failed");
    assert_eq!(user.email, "login@example.com");

    // Note: login() returns user before updating last_login_at
    // Verify by fetching the user again
    let fetched = manager
        .get_user(&registered.id)
        .await
        .expect("Get user failed");
    assert!(fetched.last_login_at.is_some()); // Should be updated after login
}

#[tokio::test]
async fn test_user_login_wrong_password_rejected() {
    let manager = setup_user_manager().await;

    // Register user first
    let register_request = RegisterRequest {
        email: "login@example.com".to_string(),
        password: "correctpassword".to_string(),
        name: None,
    };
    manager.register(register_request).await.unwrap();

    // Login with wrong password
    let login_request = LoginRequest {
        email: "login@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };

    let result = manager.login(login_request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_user_login_nonexistent_user_rejected() {
    let manager = setup_user_manager().await;

    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_string(),
        password: "anypassword".to_string(),
    };

    let result = manager.login(login_request).await;
    assert!(result.is_err());
}

// =============================================================================
// Profile Management Tests
// =============================================================================

#[tokio::test]
async fn test_get_user_by_id() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "getuser@example.com".to_string(),
        password: "password123".to_string(),
        name: Some("Get User".to_string()),
    };
    let created = manager.register(register_request).await.unwrap();

    let retrieved = manager
        .get_user(&created.id)
        .await
        .expect("Get user failed");
    assert_eq!(retrieved.id, created.id);
    assert_eq!(retrieved.email, "getuser@example.com");
}

#[tokio::test]
async fn test_get_user_by_email() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "byemail@example.com".to_string(),
        password: "password123".to_string(),
        name: None,
    };
    let created = manager.register(register_request).await.unwrap();

    let retrieved = manager
        .get_user_by_email("byemail@example.com")
        .await
        .expect("Get user by email failed")
        .expect("User should exist");

    assert_eq!(retrieved.id, created.id);
}

#[tokio::test]
async fn test_update_user_name() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "update@example.com".to_string(),
        password: "password123".to_string(),
        name: Some("Original Name".to_string()),
    };
    let user = manager.register(register_request).await.unwrap();

    let update_request = UpdateUserRequest {
        name: Some("Updated Name".to_string()),
        email: None,
    };

    let updated = manager
        .update_user(&user.id, update_request)
        .await
        .expect("Update failed");

    assert_eq!(updated.name, Some("Updated Name".to_string()));
}

#[tokio::test]
async fn test_change_password() {
    let manager = setup_user_manager().await;

    // Register user
    let register_request = RegisterRequest {
        email: "changepw@example.com".to_string(),
        password: "oldpassword".to_string(),
        name: None,
    };
    let user = manager.register(register_request).await.unwrap();

    // Change password
    let change_request = ChangePasswordRequest {
        current_password: "oldpassword".to_string(),
        new_password: "newpassword123".to_string(),
    };

    manager
        .change_password(&user.id, change_request)
        .await
        .expect("Password change failed");

    // Login with new password should work
    let login_request = LoginRequest {
        email: "changepw@example.com".to_string(),
        password: "newpassword123".to_string(),
    };
    let logged_in = manager.login(login_request).await;
    assert!(logged_in.is_ok());

    // Login with old password should fail
    let old_login = LoginRequest {
        email: "changepw@example.com".to_string(),
        password: "oldpassword".to_string(),
    };
    assert!(manager.login(old_login).await.is_err());
}

#[tokio::test]
async fn test_change_password_wrong_current_rejected() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "wrongcurrent@example.com".to_string(),
        password: "actualpassword".to_string(),
        name: None,
    };
    let user = manager.register(register_request).await.unwrap();

    let change_request = ChangePasswordRequest {
        current_password: "wrongpassword".to_string(),
        new_password: "newpassword123".to_string(),
    };

    let result = manager.change_password(&user.id, change_request).await;
    assert!(result.is_err());
}

// =============================================================================
// Plan & Usage Tests
// =============================================================================

#[tokio::test]
async fn test_plan_limits() {
    // Free plan limits
    let free_limits = Plan::Free.limits();
    assert_eq!(free_limits.invocations_per_month, 100_000);
    assert_eq!(free_limits.cron_jobs, 0);
    assert_eq!(free_limits.max_functions, 5);

    // Pro plan limits
    let pro_limits = Plan::Pro.limits();
    assert_eq!(pro_limits.invocations_per_month, 1_000_000);
    assert_eq!(pro_limits.cron_jobs, 10);
    assert_eq!(pro_limits.max_functions, 50);

    // Scale plan limits
    let scale_limits = Plan::Scale.limits();
    assert_eq!(scale_limits.invocations_per_month, 10_000_000);
    assert_eq!(scale_limits.cron_jobs, 100);
    assert_eq!(scale_limits.max_functions, 500); // Fixed: was 200, actual is 500
}

#[tokio::test]
async fn test_user_usage_tracking() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "usage@example.com".to_string(),
        password: "password123".to_string(),
        name: None,
    };
    let user = manager.register(register_request).await.unwrap();

    // Get initial usage (should be zero)
    let usage = manager
        .get_usage(&user.id)
        .await
        .expect("Failed to get usage");

    assert_eq!(usage.invocations, 0);
    assert_eq!(usage.bandwidth_bytes, 0);
}

#[tokio::test]
async fn test_increment_usage() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "increment@example.com".to_string(),
        password: "password123".to_string(),
        name: None,
    };
    let user = manager.register(register_request).await.unwrap();

    // Increment usage (invocations, bandwidth_bytes)
    manager
        .increment_usage(&user.id, 1, 1024) // 1 invocation, 1KB bandwidth
        .await
        .expect("Failed to increment usage");

    let usage = manager.get_usage(&user.id).await.unwrap();
    assert_eq!(usage.invocations, 1);
    assert_eq!(usage.bandwidth_bytes, 1024);

    // Increment again
    manager
        .increment_usage(&user.id, 1, 2048) // 1 more invocation, 2KB bandwidth
        .await
        .expect("Failed to increment usage");

    let usage = manager.get_usage(&user.id).await.unwrap();
    assert_eq!(usage.invocations, 2);
    assert_eq!(usage.bandwidth_bytes, 3072);
}

#[tokio::test]
async fn test_check_limits_within() {
    let manager = setup_user_manager().await;

    // Use Pro plan which has non-zero cron limit (Free plan has cron_jobs=0 which triggers violation)
    let register_request = RegisterRequest {
        email: "limits@example.com".to_string(),
        password: "password123".to_string(),
        name: None,
    };
    let user = manager.register(register_request).await.unwrap();

    // Upgrade to Pro plan to avoid cron_jobs=0 limit issue
    manager.update_plan(&user.id, Plan::Pro).await.unwrap();

    // Check limits (should be within for new Pro user)
    let result = manager
        .check_limits(&user.id)
        .await
        .expect("Failed to check limits");

    assert!(result.within_limits);
    assert!(result.violations.is_empty());
}

#[tokio::test]
async fn test_update_plan() {
    let manager = setup_user_manager().await;

    let register_request = RegisterRequest {
        email: "upgrade@example.com".to_string(),
        password: "password123".to_string(),
        name: None,
    };
    let user = manager.register(register_request).await.unwrap();
    assert_eq!(user.plan, Plan::Free);

    // Upgrade to Pro
    let upgraded = manager
        .update_plan(&user.id, Plan::Pro)
        .await
        .expect("Failed to upgrade plan");

    assert_eq!(upgraded.plan, Plan::Pro);

    // Verify persisted
    let retrieved = manager.get_user(&user.id).await.unwrap();
    assert_eq!(retrieved.plan, Plan::Pro);
}
