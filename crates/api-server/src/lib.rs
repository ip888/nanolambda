//! Lambda-compatible REST API

use std::sync::Arc;
use axum::{
    routing::{post, get, put, delete},
    Router,
};
use tokio::sync::Mutex;
use tracing::info;

pub mod routes;
pub mod handlers;
pub mod models;
pub mod auth;
pub mod metrics;
pub mod concurrency;
pub mod rate_limiter;
pub mod usage_tracker;

use nanolambda_runtime::{PythonExecutor, NodeJSExecutor};
use nanolambda_storage::{StorageManager, usage_db::UsageDb, pricing::PricingManager, trial::TrialManager};
use crate::metrics::MetricsCollector;
use crate::concurrency::{ConcurrencyController, ConcurrencyConfig};
use crate::rate_limiter::{RateLimiter, RateLimitTier};
use crate::usage_tracker::UsageTracker;

/// API server state
pub struct ApiServer {
    storage: Arc<StorageManager>,
    python_executor: Arc<Mutex<PythonExecutor>>,
    nodejs_executor: Arc<Mutex<NodeJSExecutor>>,
    metrics: Arc<MetricsCollector>,
    concurrency: Arc<ConcurrencyController>,
    rate_limiter: Arc<RateLimiter>,
    usage_tracker: Arc<UsageTracker>,
    usage_db: Option<Arc<UsageDb>>, // Persistent usage tracking
    pricing: Option<Arc<PricingManager>>, // Dynamic pricing configuration
    trial_manager: Option<Arc<TrialManager>>, // Trial period tracking
}

impl ApiServer {
    /// Create new API server with database path
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = StorageManager::new(db_path)?;
        let python_executor = PythonExecutor::new()?;
        let nodejs_executor = NodeJSExecutor::new()?;
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
        
        Ok(Self {
            storage: Arc::new(storage),
            python_executor: Arc::new(Mutex::new(python_executor)),
            nodejs_executor: Arc::new(Mutex::new(nodejs_executor)),
            metrics: Arc::new(MetricsCollector::new()),
            concurrency: Arc::new(ConcurrencyController::new(concurrency_config)),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitTier::Free)),
            usage_tracker: Arc::new(UsageTracker::new(10000)),
            usage_db: Some(Arc::new(usage_db)),
            pricing: Some(Arc::new(pricing)),
            trial_manager: Some(Arc::new(trial_manager)),
        })
    }
    
    /// Create new API server with in-memory database (for testing)
    pub async fn new_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let storage = StorageManager::new_in_memory()?;
        let python_executor = PythonExecutor::new()?;
        let nodejs_executor = NodeJSExecutor::new()?;
        let concurrency_config = ConcurrencyConfig::default();
        
        Ok(Self {
            storage: Arc::new(storage),
            python_executor: Arc::new(Mutex::new(python_executor)),
            nodejs_executor: Arc::new(Mutex::new(nodejs_executor)),
            metrics: Arc::new(MetricsCollector::new()),
            concurrency: Arc::new(ConcurrencyController::new(concurrency_config)),
            rate_limiter: Arc::new(RateLimiter::new(RateLimitTier::Free)),
            usage_tracker: Arc::new(UsageTracker::new(10000)),
            usage_db: None, // No persistent tracking for in-memory mode
            pricing: None, // No dynamic pricing for in-memory mode
            trial_manager: None, // No trial tracking for in-memory mode
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
    
    /// Get Node.js executor reference
    pub fn nodejs_executor(&self) -> &Arc<Mutex<NodeJSExecutor>> {
        &self.nodejs_executor
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
            
            // Function versioning
            .route("/functions/{name}/versions", get(handlers::list_function_versions))
            .route("/functions/{name}/versions", post(handlers::publish_function_version))
            .route("/functions/{name}/versions/{version}", get(handlers::get_function_version))
            
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
            
            // Pricing updates (admin only)
            .route("/pricing", put(handlers::update_pricing))
            
            .layer(axum::middleware::from_fn_with_state(
                storage_for_auth,
                auth::auth_middleware
            ))
            .with_state(state.clone());
        
        // Public routes (no auth required)
        let public_routes = Router::new()
            // API key creation (must be public to get first key)
            .route("/auth/keys", post(handlers::create_api_key))
            
            // Metrics (public for now, could be protected later)
            .route("/metrics", get(handlers::get_metrics))
            .route("/dashboard", get(handlers::get_dashboard))
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
            
            // Health check
            .route("/health", get(handlers::health_check))
            .with_state(state);
        
        let app = Router::new()
            .merge(protected_routes)
            .merge(public_routes);

        info!("Starting API server on 0.0.0.0:8080");
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, app.into_make_service())
            .await?;

        Ok(())
    }
}
