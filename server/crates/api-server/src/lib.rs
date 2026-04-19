//! Lambda-compatible REST API

use axum::http::StatusCode as AxumStatusCode;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tracing::info;

pub mod analytics_handlers;
pub mod annual_handlers;
pub mod auth;
pub mod churn_handlers;
pub mod clv_handlers;
pub mod concurrency;
pub mod discount_handlers;
pub mod handlers;
pub mod metrics;
pub mod models;
pub mod payment_retry_handlers;
pub mod rate_limiter;
pub mod referral_handlers;
pub mod routes;
pub mod sandbox_handlers;
pub mod scheduler_handlers;
pub mod sse_handlers;
pub mod usage_tracker;
pub mod user_handlers;
pub mod websocket_handlers;

use crate::concurrency::{ConcurrencyConfig, ConcurrencyController};
use crate::metrics::MetricsCollector;
use crate::rate_limiter::{RateLimitTier, RateLimiter};
use crate::usage_tracker::UsageTracker;
use nanolambda_runtime::PythonExecutor;
use nanolambda_storage::{
    JwtManager, SchedulerManager, StorageManager, UserManager, payment::PaymentManager,
    pricing::PricingManager, tier::TierManager, trial::TrialManager, usage_db::UsageDb,
};

/// API server state
pub struct ApiServer {
    storage: Arc<StorageManager>,
    python_executor: Arc<Mutex<PythonExecutor>>,
    metrics: Arc<MetricsCollector>,
    concurrency: Arc<ConcurrencyController>,
    rate_limiter: Arc<RateLimiter>,
    usage_tracker: Arc<UsageTracker>,
    usage_db: Option<Arc<UsageDb>>,       // Persistent usage tracking
    pricing: Option<Arc<PricingManager>>, // Dynamic pricing configuration
    trial_manager: Option<Arc<TrialManager>>, // Trial period tracking
    tier_manager: Option<Arc<TierManager>>, // Tiered pricing management
    payment_manager: Option<Arc<PaymentManager>>, // Stripe payment integration
    invoice_manager: Arc<nanolambda_storage::invoice::InvoiceManager>, // Invoice generation and storage
    discount_manager: Option<Arc<nanolambda_storage::discount::DiscountManager>>, // Discount code management
    referral_manager: Option<Arc<nanolambda_storage::referral::ReferralManager>>, // Referral program management
    annual_billing_manager: Option<Arc<nanolambda_storage::annual::AnnualBillingManager>>, // Annual billing management
    analytics_manager: Option<Arc<nanolambda_storage::analytics::UsageAnalyticsManager>>, // Usage analytics and insights
    clv_manager: Option<Arc<nanolambda_storage::clv::CLVManager>>, // Customer Lifetime Value tracking
    churn_analyzer: Option<Arc<nanolambda_storage::churn::ChurnAnalyzer>>, // Churn analysis and prevention
    payment_retry_manager: Option<Arc<nanolambda_storage::payment_retry::PaymentRetryManager>>, // Payment retry logic
    // Phase 1-2: User authentication and scheduling
    user_manager: Option<Arc<UserManager>>, // User account management
    jwt_manager: Option<Arc<JwtManager>>,   // JWT token management
    scheduler_manager: Option<Arc<SchedulerManager>>, // Scheduled jobs (cron)
}

impl ApiServer {
    /// Create new API server with database path
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = StorageManager::new(db_path)?;
        let python_executor = PythonExecutor::new()?;

        let concurrency_config = ConcurrencyConfig::default();

        // Create separate usage database
        let usage_db_path = format!("{}.usage.db", db_path);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}?mode=rwc", usage_db_path))
            .await?;
        let usage_db = UsageDb::new(pool.clone()).await?;
        let pricing = PricingManager::new(pool.clone()).await?;
        let trial_manager = TrialManager::new(pool.clone()).await?;
        let tier_manager = TierManager::new(pool.clone()).await?;

        // Initialize payment manager if Stripe key is provided
        let stripe_key =
            std::env::var("STRIPE_SECRET_KEY").unwrap_or_else(|_| "sk_test_dummy".to_string());

        let payment_manager = if stripe_key != "sk_test_dummy" {
            Some(Arc::new(
                PaymentManager::new(stripe_key.clone(), pool.clone()).await?,
            ))
        } else {
            info!("STRIPE_SECRET_KEY not set - payment features disabled");
            None
        };

        // Initialize invoice manager (always available)
        let invoice_manager = Arc::new(
            nanolambda_storage::invoice::InvoiceManager::new(stripe_key.clone(), pool.clone())
                .await?,
        );

        // Initialize discount manager (always available)
        let discount_manager = Arc::new(
            nanolambda_storage::discount::DiscountManager::new(stripe_key.clone(), pool.clone())
                .await?,
        );

        // Initialize referral manager (always available)
        let referral_manager =
            Arc::new(nanolambda_storage::referral::ReferralManager::new(pool.clone()).await?);

        // Initialize annual billing manager (always available)
        let annual_billing_manager =
            Arc::new(nanolambda_storage::annual::AnnualBillingManager::new(pool.clone()).await?);

        // Initialize analytics manager (always available)
        let analytics_manager = Arc::new(
            nanolambda_storage::analytics::UsageAnalyticsManager::new(pool.clone()).await?,
        );

        // Initialize CLV manager (always available)
        let clv_manager = Arc::new(nanolambda_storage::clv::CLVManager::new(pool.clone()).await?);

        // Initialize churn analyzer (always available)
        let churn_analyzer =
            Arc::new(nanolambda_storage::churn::ChurnAnalyzer::new(pool.clone()).await?);

        // Initialize payment retry manager (always available)
        let payment_retry_manager =
            Arc::new(nanolambda_storage::payment_retry::PaymentRetryManager::new().await?);

        // Phase 1-2: Initialize user management, JWT, and scheduler
        let user_manager = Arc::new(UserManager::new(pool.clone()).await?);
        let jwt_manager = Arc::new(JwtManager::new(pool.clone()).await?);
        let scheduler_manager = Arc::new(SchedulerManager::new(pool.clone()).await?);

        info!("User management and scheduler initialized");

        Ok(Self {
            storage: Arc::new(storage),
            python_executor: Arc::new(Mutex::new(python_executor)),
            metrics: Arc::new(MetricsCollector::new()),
            concurrency: Arc::new(ConcurrencyController::new(concurrency_config)),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitTier::Free)),
            usage_tracker: Arc::new(UsageTracker::new(10000)),
            usage_db: Some(Arc::new(usage_db)),
            pricing: Some(Arc::new(pricing)),
            trial_manager: Some(Arc::new(trial_manager)),
            tier_manager: Some(Arc::new(tier_manager)),
            payment_manager,
            invoice_manager,
            discount_manager: Some(discount_manager),
            referral_manager: Some(referral_manager),
            annual_billing_manager: Some(annual_billing_manager),
            analytics_manager: Some(analytics_manager),
            clv_manager: Some(clv_manager),
            churn_analyzer: Some(churn_analyzer),
            payment_retry_manager: Some(payment_retry_manager),
            user_manager: Some(user_manager),
            jwt_manager: Some(jwt_manager),
            scheduler_manager: Some(scheduler_manager),
        })
    }

    /// Create new API server with in-memory database (for testing)
    pub async fn new_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let storage = StorageManager::new_in_memory()?;
        let python_executor = PythonExecutor::new()?;
        let concurrency_config = ConcurrencyConfig::default();

        // Create in-memory invoice manager for testing
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let invoice_manager = Arc::new(
            nanolambda_storage::invoice::InvoiceManager::new(
                "sk_test_dummy".to_string(),
                pool.clone(),
            )
            .await?,
        );
        let discount_manager = Arc::new(
            nanolambda_storage::discount::DiscountManager::new(
                "sk_test_dummy".to_string(),
                pool.clone(),
            )
            .await?,
        );
        let referral_manager =
            Arc::new(nanolambda_storage::referral::ReferralManager::new(pool.clone()).await?);
        let annual_billing_manager =
            Arc::new(nanolambda_storage::annual::AnnualBillingManager::new(pool.clone()).await?);
        let analytics_manager = Arc::new(
            nanolambda_storage::analytics::UsageAnalyticsManager::new(pool.clone()).await?,
        );
        let clv_manager = Arc::new(nanolambda_storage::clv::CLVManager::new(pool.clone()).await?);
        let churn_analyzer =
            Arc::new(nanolambda_storage::churn::ChurnAnalyzer::new(pool.clone()).await?);
        let payment_retry_manager =
            Arc::new(nanolambda_storage::payment_retry::PaymentRetryManager::new().await?);

        // Phase 1-2: Initialize user management, JWT, and scheduler for in-memory mode
        let user_manager = Arc::new(UserManager::new(pool.clone()).await?);
        let jwt_manager = Arc::new(JwtManager::new(pool.clone()).await?);
        let scheduler_manager = Arc::new(SchedulerManager::new(pool.clone()).await?);

        Ok(Self {
            storage: Arc::new(storage),
            python_executor: Arc::new(Mutex::new(python_executor)),
            metrics: Arc::new(MetricsCollector::new()),
            concurrency: Arc::new(ConcurrencyController::new(concurrency_config)),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitTier::Free)),
            usage_tracker: Arc::new(UsageTracker::new(10000)),
            usage_db: None,        // No persistent tracking for in-memory mode
            pricing: None,         // No dynamic pricing for in-memory mode
            trial_manager: None,   // No trial tracking for in-memory mode
            tier_manager: None,    // No tier management for in-memory mode
            payment_manager: None, // No payment processing for in-memory mode
            invoice_manager,
            discount_manager: Some(discount_manager),
            referral_manager: Some(referral_manager),
            annual_billing_manager: Some(annual_billing_manager),
            analytics_manager: Some(analytics_manager),
            clv_manager: Some(clv_manager),
            churn_analyzer: Some(churn_analyzer),
            payment_retry_manager: Some(payment_retry_manager),
            user_manager: Some(user_manager),
            jwt_manager: Some(jwt_manager),
            scheduler_manager: Some(scheduler_manager),
        })
    }

    /// Get storage reference
    pub fn storage(&self) -> &Arc<StorageManager> {
        &self.storage
    }

    /// Get Python executor reference
    pub fn python_executor(&self) -> &Arc<Mutex<PythonExecutor>> {
        &self.python_executor
    }

    /// Get metrics collector reference
    pub fn metrics(&self) -> &Arc<MetricsCollector> {
        &self.metrics
    }

    /// Get concurrency controller reference
    pub fn concurrency(&self) -> &Arc<ConcurrencyController> {
        &self.concurrency
    }

    /// Get rate limiter reference
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }

    /// Get usage tracker reference
    pub fn usage_tracker(&self) -> &Arc<UsageTracker> {
        &self.usage_tracker
    }

    /// Get usage database reference (persistent tracking)
    pub fn usage_db(&self) -> Option<&Arc<UsageDb>> {
        self.usage_db.as_ref()
    }

    /// Get pricing manager reference (dynamic pricing)
    pub fn pricing(&self) -> Option<&Arc<PricingManager>> {
        self.pricing.as_ref()
    }

    /// Get trial manager reference (trial period tracking)
    pub fn trial_manager(&self) -> Option<&Arc<TrialManager>> {
        self.trial_manager.as_ref()
    }

    /// Get tier manager reference (tiered pricing management)
    pub fn tier_manager(&self) -> Option<&Arc<TierManager>> {
        self.tier_manager.as_ref()
    }

    /// Get payment manager reference (Stripe payment integration)
    pub fn payment_manager(&self) -> Option<&Arc<PaymentManager>> {
        self.payment_manager.as_ref()
    }

    /// Get discount manager reference (discount code management)
    pub fn discount_manager(&self) -> Option<&Arc<nanolambda_storage::discount::DiscountManager>> {
        self.discount_manager.as_ref()
    }

    /// Get referral manager reference (referral program management)
    pub fn referral_manager(&self) -> Option<&Arc<nanolambda_storage::referral::ReferralManager>> {
        self.referral_manager.as_ref()
    }

    /// Get annual billing manager reference (annual subscription management)
    pub fn annual_billing_manager(
        &self,
    ) -> Option<&Arc<nanolambda_storage::annual::AnnualBillingManager>> {
        self.annual_billing_manager.as_ref()
    }

    /// Get analytics manager reference (usage analytics and insights)
    pub fn analytics_manager(
        &self,
    ) -> Option<&Arc<nanolambda_storage::analytics::UsageAnalyticsManager>> {
        self.analytics_manager.as_ref()
    }

    /// Get CLV manager reference (customer lifetime value tracking)
    pub fn clv_manager(&self) -> Option<&Arc<nanolambda_storage::clv::CLVManager>> {
        self.clv_manager.as_ref()
    }

    /// Get churn analyzer reference (churn analysis and prevention)
    pub fn churn_analyzer(&self) -> Option<&Arc<nanolambda_storage::churn::ChurnAnalyzer>> {
        self.churn_analyzer.as_ref()
    }

    /// Get payment retry manager reference (payment retry logic)
    pub fn payment_retry_manager(
        &self,
    ) -> Option<&Arc<nanolambda_storage::payment_retry::PaymentRetryManager>> {
        self.payment_retry_manager.as_ref()
    }

    /// Get user manager reference (user account management)
    pub fn get_user_manager(&self) -> Option<Arc<UserManager>> {
        self.user_manager.clone()
    }

    /// Get JWT manager reference (token management)
    pub fn get_jwt_manager(&self) -> Option<Arc<JwtManager>> {
        self.jwt_manager.clone()
    }

    /// Get scheduler manager reference (scheduled jobs / cron)
    pub fn get_scheduler_manager(&self) -> Option<Arc<SchedulerManager>> {
        self.scheduler_manager.clone()
    }

    /// Invoke a function by name (used by scheduler)
    pub async fn invoke_function_by_name(
        &self,
        function_name: &str,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let function = self
            .storage
            .get_function(function_name)?
            .ok_or_else(|| format!("Function '{}' not found", function_name))?;
        let payload = payload.unwrap_or(serde_json::json!({}));

        // Python is the only supported runtime after the 2026-04 niche refocus.
        let result = if function.runtime.starts_with("python") {
            let config = nanolambda_runtime::FunctionConfig {
                id: function.id,
                version: function.version,
                name: function.name.clone(),
                code: function.code.clone(),
                handler: function.handler.clone(),
                environment: function.environment.clone(),
                memory_limit_mb: function.memory_mb,
                timeout_seconds: function.timeout_ms / 1000,
                working_dir: None,
            };
            let executor = self.python_executor.lock().await;
            executor.execute(config, payload)?
        } else {
            return Err(format!("Unsupported runtime: {}", function.runtime).into());
        };

        Ok(serde_json::json!({
            "statusCode": if result.success { 200 } else { 500 },
            "body": result.result.unwrap_or_default(),
            "error": result.error
        }))
    }

    /// Start the API server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let state = Arc::new(self);

        // Clone storage for auth middleware
        let storage_for_auth = state.storage.clone();

        // Protected routes (require authentication)
        let protected_routes = Router::new()
            // Function management
            .route("/functions", post(handlers::create_function))
            .route("/functions", get(handlers::list_functions))
            .route("/functions/{name}", get(handlers::get_function))
            .route("/functions/{name}", put(handlers::update_function))
            .route("/functions/{name}", delete(handlers::delete_function))
            // Function invocation
            .route("/functions/{name}/invoke", post(handlers::invoke_function))
            // SSE streaming endpoints
            .route(
                "/functions/{name}/stream",
                post(sse_handlers::stream_function),
            )
            .route("/stream/fanout", post(sse_handlers::stream_fanout))
            // Function versioning
            .route(
                "/functions/{name}/versions",
                get(handlers::list_function_versions),
            )
            .route(
                "/functions/{name}/versions",
                post(handlers::publish_function_version),
            )
            .route(
                "/functions/{name}/versions/{version}",
                get(handlers::get_function_version),
            )
            // API Key management (viewing/revoking requires auth)
            .route("/auth/keys", get(handlers::list_api_keys))
            .route("/auth/keys/{id}", delete(handlers::revoke_api_key))
            // Rate limiting status (requires auth)
            .route("/rate-limit/status", get(handlers::get_rate_limit_status))
            // Usage tracking and billing (requires auth)
            .route("/usage/stats", get(handlers::get_usage_stats))
            .route("/usage/billing", get(handlers::get_billing_info))
            // Trial status (requires auth to view own trial)
            .route("/trial/status", get(handlers::get_trial_status))
            // Tier management (requires auth to view own tier and upgrade)
            .route("/tier/current", get(handlers::get_current_tier))
            .route("/tier/upgrade", put(handlers::upgrade_tier))
            .route(
                "/tier/recommendation",
                get(handlers::get_upgrade_recommendation),
            )
            .route("/tier/preview", get(handlers::get_upgrade_preview))
            // Invoice management (requires auth)
            .route("/invoices", get(handlers::list_invoices))
            .route("/invoices/summary", get(handlers::get_invoice_summary))
            .route("/invoices/{invoice_id}", get(handlers::get_invoice))
            .route(
                "/invoices/sync/{stripe_invoice_id}",
                post(handlers::sync_invoice),
            )
            // Payment management (Stripe integration - requires auth)
            .route("/payment/customer", post(handlers::create_customer))
            .route("/payment/customer", get(handlers::get_customer_info))
            .route("/payment/method", post(handlers::attach_payment_method))
            .route("/payment/subscription", post(handlers::create_subscription))
            .route(
                "/payment/subscription",
                delete(handlers::cancel_subscription),
            )
            // Metered billing (requires auth)
            .route("/payment/usage", post(handlers::report_metered_usage))
            .route("/payment/overage", get(handlers::calculate_overage))
            // Customer Portal (requires auth)
            .route("/payment/portal", post(handlers::create_portal_session))
            // Usage alerts (requires auth)
            .route("/usage/check-alerts", post(handlers::check_usage_alerts))
            .route("/usage/alerts", get(handlers::get_usage_alerts))
            // Discount code management (requires auth)
            .route("/discounts/apply", post(discount_handlers::apply_discount))
            .route(
                "/discounts/my-usage",
                get(discount_handlers::get_user_discount_usage),
            )
            // Admin discount routes (requires ADMIN_API_KEY)
            .route("/discounts", post(discount_handlers::create_discount))
            .route("/discounts", get(discount_handlers::list_discounts))
            .route(
                "/discounts/{discount_id}/usage",
                get(discount_handlers::get_discount_usage),
            )
            // Referral program management (requires auth)
            .route(
                "/referrals/generate",
                post(referral_handlers::generate_referral_code),
            )
            .route(
                "/referrals/my-code",
                get(referral_handlers::get_my_referral_code),
            )
            .route(
                "/referrals/my-rewards",
                get(referral_handlers::get_my_referral_rewards),
            )
            // Annual billing management (requires auth)
            .route(
                "/billing/annual/plan",
                get(annual_handlers::get_annual_plan),
            )
            .route(
                "/billing/annual/upgrade",
                post(annual_handlers::upgrade_to_annual),
            )
            .route(
                "/billing/annual/usage",
                get(annual_handlers::get_annual_subscription_usage),
            )
            .route(
                "/billing/annual/downgrade",
                post(annual_handlers::downgrade_from_annual),
            )
            // Usage analytics (requires auth)
            .route(
                "/analytics/daily-summaries",
                get(analytics_handlers::get_daily_summaries),
            )
            .route(
                "/analytics/profile",
                get(analytics_handlers::get_usage_profile),
            )
            .route(
                "/analytics/snapshot",
                post(analytics_handlers::get_usage_snapshot),
            )
            .route(
                "/analytics/trends",
                get(analytics_handlers::get_monthly_trends),
            )
            // Customer Lifetime Value (requires auth)
            .route("/clv", get(clv_handlers::get_customer_clv))
            .route("/clv/calculate", post(clv_handlers::calculate_clv))
            .route("/clv/predict", post(clv_handlers::get_revenue_prediction))
            .route("/clv/cohorts", get(clv_handlers::get_cohort_analysis))
            // Churn analysis and prevention (requires auth)
            .route("/churn/analyze", post(churn_handlers::analyze_churn_risk))
            .route("/churn/risk", get(churn_handlers::get_risk_profile))
            .route("/churn/record", post(churn_handlers::record_churn))
            .route(
                "/churn/intervention",
                post(churn_handlers::record_intervention),
            )
            .route(
                "/churn/interventions",
                get(churn_handlers::get_interventions),
            )
            // Payment retry logic (requires auth)
            .route(
                "/payment-retry/record-failure",
                post(payment_retry_handlers::record_payment_failure),
            )
            .route(
                "/payment-retry/process",
                post(payment_retry_handlers::process_retry),
            )
            .route(
                "/payment-retry/status",
                get(payment_retry_handlers::get_retry_status),
            )
            .route(
                "/payment-retry/clear",
                post(payment_retry_handlers::clear_retry_status),
            )
            .route(
                "/payment-retry/send-dunning",
                post(payment_retry_handlers::send_dunning_notification),
            )
            // Scheduled jobs (cron) - requires auth
            .route("/schedules", post(scheduler_handlers::create_schedule))
            .route("/schedules", get(scheduler_handlers::list_schedules))
            .route("/schedules/{id}", get(scheduler_handlers::get_schedule))
            .route("/schedules/{id}", put(scheduler_handlers::update_schedule))
            .route(
                "/schedules/{id}",
                delete(scheduler_handlers::delete_schedule),
            )
            .route(
                "/schedules/{id}/runs",
                get(scheduler_handlers::get_schedule_runs),
            )
            .route(
                "/schedules/{id}/trigger",
                post(scheduler_handlers::trigger_schedule),
            )
            // User profile (requires auth)
            .route("/user/me", get(user_handlers::get_me))
            .route("/user/me", put(user_handlers::update_me))
            .route("/user/password", put(user_handlers::change_password))
            .route("/user/usage", get(user_handlers::get_usage))
            .route("/user/limits", get(user_handlers::check_limits))
            .route("/user/sessions", get(user_handlers::get_sessions))
            .route("/user/sessions/{id}", delete(user_handlers::revoke_session))
            // WebSocket info (requires auth)
            .route("/ws/info", get(websocket_handlers::get_ws_info))
            // Pricing updates (admin only)
            .route("/pricing", put(handlers::update_pricing))
            .layer(axum::middleware::from_fn_with_state(
                storage_for_auth,
                auth::auth_middleware,
            ))
            .with_state(state.clone());

        // Public routes (no auth required)
        let public_routes = Router::new()
            // User authentication (public - must be public for login/register)
            .route("/auth/register", post(user_handlers::register))
            .route("/auth/login", post(user_handlers::login))
            .route("/auth/logout", post(user_handlers::logout))
            .route("/auth/refresh", post(user_handlers::refresh_token))
            // API key creation (must be public to get first key)
            .route("/auth/keys", post(handlers::create_api_key))
            // Metrics (public for now, could be protected later)
            .route("/metrics", get(handlers::get_metrics))
            .route("/dashboard", get(handlers::get_dashboard))
            .route("/dashboard/{*file}", get(handlers::get_dashboard_file))
            .route("/concurrency", get(handlers::get_concurrency_stats))
            // Rate limiting admin endpoints (public for now, should be protected)
            .route("/rate-limit/stats", get(handlers::get_all_rate_limit_stats))
            .route("/rate-limit/tier", put(handlers::set_rate_limit_tier))
            // Usage tracking admin endpoints (public for now, should be protected)
            .route("/usage/all", get(handlers::get_all_usage_stats))
            // Pricing (public - anyone can view current rates)
            .route("/pricing", get(handlers::get_pricing))
            .route("/pricing/history", get(handlers::get_pricing_history))
            // Trial admin endpoints (public for now, should be protected)
            .route("/trial/all", get(handlers::get_all_trials))
            // Tier plans (public - anyone can view available tiers)
            .route("/tier/plans", get(handlers::get_tier_plans))
            // Stripe webhooks (public - verified by signature)
            .route("/webhooks/stripe", post(handlers::stripe_webhook))
            // Discount validation (public - no auth required for validation)
            .route(
                "/discounts/validate",
                post(discount_handlers::validate_discount),
            )
            // Referral tracking and details (public)
            .route(
                "/referrals/track",
                post(referral_handlers::track_referral_click),
            )
            .route(
                "/referrals/leaderboard",
                get(referral_handlers::get_leaderboard),
            )
            .route(
                "/referrals/{code}",
                get(referral_handlers::get_referral_details),
            )
            // Annual billing pricing (public)
            .route(
                "/billing/annual/pricing/{tier}",
                get(annual_handlers::get_annual_pricing),
            )
            // Platform analytics (public - admin can view)
            .route(
                "/analytics/platform",
                get(analytics_handlers::get_platform_analytics),
            )
            // CLV segments and summary (public - admin view)
            .route("/clv/segments", get(clv_handlers::get_clv_segments))
            .route("/clv/summary", get(clv_handlers::get_platform_clv_summary))
            // Churn predictions and platform metrics (public - admin view)
            .route("/churn/predict", get(churn_handlers::predict_churn))
            .route("/churn/metrics", get(churn_handlers::get_platform_metrics))
            // Payment retry metrics and past-due customers (public - admin view)
            .route(
                "/payment-retry/metrics",
                get(payment_retry_handlers::get_platform_metrics),
            )
            .route(
                "/payment-retry/past-due",
                get(payment_retry_handlers::get_past_due_customers),
            )
            // WebSocket endpoint (public - auth via query param)
            .route(
                "/ws/{function_name}",
                get(websocket_handlers::websocket_handler),
            )
            // Sandbox (MCP tool-exec endpoint; auth handled by MCP daemon)
            .route("/sandbox/invoke", post(sandbox_handlers::sandbox_invoke))
            // Health check
            .route("/health", get(handlers::health_check))
            .with_state(state);

        // CORS configuration - use allowed origins from env or default to permissive
        let cors = match std::env::var("CORS_ALLOWED_ORIGINS") {
            Ok(origins) => {
                let allowed: Vec<_> = origins
                    .split(',')
                    .filter_map(|o| o.trim().parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(allowed)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
            Err(_) => {
                // Default: permissive for development
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
        };

        let app = Router::new()
            .merge(protected_routes)
            .merge(public_routes)
            .layer(cors);

        info!("Starting API server on 0.0.0.0:8080");
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, app.into_make_service()).await?;

        Ok(())
    }

    /// Start the API server with graceful shutdown support
    pub async fn run_with_shutdown(
        self,
        host: &str,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = Arc::new(self);

        // Clone storage for auth middleware
        let storage_for_auth = state.storage.clone();

        // Protected routes (require authentication)
        let protected_routes = Router::new()
            // Function management
            .route("/functions", post(handlers::create_function))
            .route("/functions", get(handlers::list_functions))
            .route("/functions/{name}", get(handlers::get_function))
            .route("/functions/{name}", put(handlers::update_function))
            .route("/functions/{name}", delete(handlers::delete_function))
            // Function invocation
            .route("/functions/{name}/invoke", post(handlers::invoke_function))
            // SSE streaming endpoints
            .route(
                "/functions/{name}/stream",
                post(sse_handlers::stream_function),
            )
            .route("/stream/fanout", post(sse_handlers::stream_fanout))
            // Function versioning
            .route(
                "/functions/{name}/versions",
                get(handlers::list_function_versions),
            )
            .route(
                "/functions/{name}/versions",
                post(handlers::publish_function_version),
            )
            .route(
                "/functions/{name}/versions/{version}",
                get(handlers::get_function_version),
            )
            // API Key management
            .route("/auth/keys", get(handlers::list_api_keys))
            .route("/auth/keys/{id}", delete(handlers::revoke_api_key))
            // Rate limiting status
            .route("/rate-limit/status", get(handlers::get_rate_limit_status))
            // Usage tracking and billing
            .route("/usage/stats", get(handlers::get_usage_stats))
            .route("/usage/billing", get(handlers::get_billing_info))
            // Trial status
            .route("/trial/status", get(handlers::get_trial_status))
            // Tier management
            .route("/tier/current", get(handlers::get_current_tier))
            .route("/tier/upgrade", put(handlers::upgrade_tier))
            .route(
                "/tier/recommendation",
                get(handlers::get_upgrade_recommendation),
            )
            .route("/tier/preview", get(handlers::get_upgrade_preview))
            // Invoice management
            .route("/invoices", get(handlers::list_invoices))
            .route("/invoices/summary", get(handlers::get_invoice_summary))
            .route("/invoices/{invoice_id}", get(handlers::get_invoice))
            .route(
                "/invoices/sync/{stripe_invoice_id}",
                post(handlers::sync_invoice),
            )
            // Payment management
            .route("/payment/customer", post(handlers::create_customer))
            .route("/payment/customer", get(handlers::get_customer_info))
            .route("/payment/method", post(handlers::attach_payment_method))
            .route("/payment/subscription", post(handlers::create_subscription))
            .route(
                "/payment/subscription",
                delete(handlers::cancel_subscription),
            )
            // Metered billing
            .route("/payment/usage", post(handlers::report_metered_usage))
            .route("/payment/overage", get(handlers::calculate_overage))
            // Customer Portal
            .route("/payment/portal", post(handlers::create_portal_session))
            // Usage alerts
            .route("/usage/check-alerts", post(handlers::check_usage_alerts))
            .route("/usage/alerts", get(handlers::get_usage_alerts))
            // Discount code management
            .route("/discounts/apply", post(discount_handlers::apply_discount))
            .route(
                "/discounts/my-usage",
                get(discount_handlers::get_user_discount_usage),
            )
            // Admin discount routes
            .route("/discounts", post(discount_handlers::create_discount))
            .route("/discounts", get(discount_handlers::list_discounts))
            .route(
                "/discounts/{discount_id}/usage",
                get(discount_handlers::get_discount_usage),
            )
            // Referral program management
            .route(
                "/referrals/generate",
                post(referral_handlers::generate_referral_code),
            )
            .route(
                "/referrals/my-code",
                get(referral_handlers::get_my_referral_code),
            )
            .route(
                "/referrals/my-rewards",
                get(referral_handlers::get_my_referral_rewards),
            )
            // Annual billing management
            .route(
                "/billing/annual/plan",
                get(annual_handlers::get_annual_plan),
            )
            .route(
                "/billing/annual/upgrade",
                post(annual_handlers::upgrade_to_annual),
            )
            .route(
                "/billing/annual/usage",
                get(annual_handlers::get_annual_subscription_usage),
            )
            .route(
                "/billing/annual/downgrade",
                post(annual_handlers::downgrade_from_annual),
            )
            // Usage analytics
            .route(
                "/analytics/daily-summaries",
                get(analytics_handlers::get_daily_summaries),
            )
            .route(
                "/analytics/profile",
                get(analytics_handlers::get_usage_profile),
            )
            .route(
                "/analytics/snapshot",
                post(analytics_handlers::get_usage_snapshot),
            )
            .route(
                "/analytics/trends",
                get(analytics_handlers::get_monthly_trends),
            )
            // Customer Lifetime Value
            .route("/clv", get(clv_handlers::get_customer_clv))
            .route("/clv/calculate", post(clv_handlers::calculate_clv))
            .route("/clv/predict", post(clv_handlers::get_revenue_prediction))
            .route("/clv/cohorts", get(clv_handlers::get_cohort_analysis))
            // Churn analysis and prevention
            .route("/churn/analyze", post(churn_handlers::analyze_churn_risk))
            .route("/churn/risk", get(churn_handlers::get_risk_profile))
            .route("/churn/record", post(churn_handlers::record_churn))
            .route(
                "/churn/intervention",
                post(churn_handlers::record_intervention),
            )
            .route(
                "/churn/interventions",
                get(churn_handlers::get_interventions),
            )
            // Payment retry logic
            .route(
                "/payment-retry/record-failure",
                post(payment_retry_handlers::record_payment_failure),
            )
            .route(
                "/payment-retry/process",
                post(payment_retry_handlers::process_retry),
            )
            .route(
                "/payment-retry/status",
                get(payment_retry_handlers::get_retry_status),
            )
            .route(
                "/payment-retry/clear",
                post(payment_retry_handlers::clear_retry_status),
            )
            .route(
                "/payment-retry/send-dunning",
                post(payment_retry_handlers::send_dunning_notification),
            )
            // Scheduled jobs (cron)
            .route("/schedules", post(scheduler_handlers::create_schedule))
            .route("/schedules", get(scheduler_handlers::list_schedules))
            .route("/schedules/{id}", get(scheduler_handlers::get_schedule))
            .route("/schedules/{id}", put(scheduler_handlers::update_schedule))
            .route(
                "/schedules/{id}",
                delete(scheduler_handlers::delete_schedule),
            )
            .route(
                "/schedules/{id}/runs",
                get(scheduler_handlers::get_schedule_runs),
            )
            .route(
                "/schedules/{id}/trigger",
                post(scheduler_handlers::trigger_schedule),
            )
            // User profile
            .route("/user/me", get(user_handlers::get_me))
            .route("/user/me", put(user_handlers::update_me))
            .route("/user/password", put(user_handlers::change_password))
            .route("/user/usage", get(user_handlers::get_usage))
            .route("/user/limits", get(user_handlers::check_limits))
            .route("/user/sessions", get(user_handlers::get_sessions))
            .route("/user/sessions/{id}", delete(user_handlers::revoke_session))
            // WebSocket info
            .route("/ws/info", get(websocket_handlers::get_ws_info))
            // Pricing updates (admin only)
            .route("/pricing", put(handlers::update_pricing))
            .layer(axum::middleware::from_fn_with_state(
                storage_for_auth,
                auth::auth_middleware,
            ))
            .with_state(state.clone());

        // Public routes (no auth required) - same as run()
        let public_routes = Router::new()
            .route("/auth/register", post(user_handlers::register))
            .route("/auth/login", post(user_handlers::login))
            .route("/auth/logout", post(user_handlers::logout))
            .route("/auth/refresh", post(user_handlers::refresh_token))
            .route("/auth/keys", post(handlers::create_api_key))
            .route("/metrics", get(handlers::get_metrics))
            .route("/dashboard", get(handlers::get_dashboard))
            .route("/dashboard/{*file}", get(handlers::get_dashboard_file))
            .route("/concurrency", get(handlers::get_concurrency_stats))
            .route("/rate-limit/stats", get(handlers::get_all_rate_limit_stats))
            .route("/rate-limit/tier", put(handlers::set_rate_limit_tier))
            .route("/usage/all", get(handlers::get_all_usage_stats))
            .route("/pricing", get(handlers::get_pricing))
            .route("/pricing/history", get(handlers::get_pricing_history))
            .route("/trial/all", get(handlers::get_all_trials))
            .route("/tier/plans", get(handlers::get_tier_plans))
            .route("/webhooks/stripe", post(handlers::stripe_webhook))
            .route(
                "/discounts/validate",
                post(discount_handlers::validate_discount),
            )
            .route(
                "/referrals/track",
                post(referral_handlers::track_referral_click),
            )
            .route(
                "/referrals/leaderboard",
                get(referral_handlers::get_leaderboard),
            )
            .route(
                "/referrals/{code}",
                get(referral_handlers::get_referral_details),
            )
            .route(
                "/billing/annual/pricing/{tier}",
                get(annual_handlers::get_annual_pricing),
            )
            .route(
                "/analytics/platform",
                get(analytics_handlers::get_platform_analytics),
            )
            .route("/clv/segments", get(clv_handlers::get_clv_segments))
            .route("/clv/summary", get(clv_handlers::get_platform_clv_summary))
            .route("/churn/predict", get(churn_handlers::predict_churn))
            .route("/churn/metrics", get(churn_handlers::get_platform_metrics))
            .route(
                "/payment-retry/metrics",
                get(payment_retry_handlers::get_platform_metrics),
            )
            .route(
                "/payment-retry/past-due",
                get(payment_retry_handlers::get_past_due_customers),
            )
            .route(
                "/ws/{function_name}",
                get(websocket_handlers::websocket_handler),
            )
            .route("/sandbox/invoke", post(sandbox_handlers::sandbox_invoke))
            .route("/health", get(handlers::health_check))
            .with_state(state);

        // CORS configuration
        let cors = match std::env::var("CORS_ALLOWED_ORIGINS") {
            Ok(origins) => {
                let allowed: Vec<_> = origins
                    .split(',')
                    .filter_map(|o| o.trim().parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(allowed)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
            Err(_) => CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        };

        let app = Router::new()
            .merge(protected_routes)
            .merge(public_routes)
            .layer(cors)
            .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MB max request body
            .layer(TimeoutLayer::with_status_code(
                AxumStatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(60),
            )); // 60s request timeout

        let bind_addr = format!("{}:{}", host, port);
        info!("Starting API server on {}", bind_addr);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

        // Graceful shutdown: wait for SIGTERM or SIGINT
        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

/// Wait for SIGTERM or SIGINT for graceful shutdown (K8s, Docker, systemd)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => { info!("Received SIGINT, starting graceful shutdown..."); },
        () = terminate => { info!("Received SIGTERM, starting graceful shutdown..."); },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_server_initialization() {
        let result = ApiServer::new_in_memory().await;
        assert!(
            result.is_ok(),
            "API server should initialize in memory mode"
        );
    }

    #[tokio::test]
    async fn test_api_server_components_accessible() {
        let server = ApiServer::new_in_memory().await.unwrap();

        // Verify core components are accessible
        let _storage = server.storage();
        let _python = server.python_executor();
        let _metrics = server.metrics();
        let _concurrency = server.concurrency();
        let _rate_limiter = server.rate_limiter();
        let _tracker = server.usage_tracker();

        // Test passed if no panics occurred
    }
}
