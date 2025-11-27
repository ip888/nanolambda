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

use nanolambda_runtime::{PythonExecutor, NodeJSExecutor};
use nanolambda_storage::StorageManager;
use crate::metrics::MetricsCollector;

/// API server state
pub struct ApiServer {
    storage: Arc<StorageManager>,
    python_executor: Arc<Mutex<PythonExecutor>>,
    nodejs_executor: Arc<Mutex<NodeJSExecutor>>,
    metrics: Arc<MetricsCollector>,
}

impl ApiServer {
    /// Create new API server with database path
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = StorageManager::new(db_path)?;
        let python_executor = PythonExecutor::new()?;
        let nodejs_executor = NodeJSExecutor::new()?;
        
        Ok(Self {
            storage: Arc::new(storage),
            python_executor: Arc::new(Mutex::new(python_executor)),
            nodejs_executor: Arc::new(Mutex::new(nodejs_executor)),
            metrics: Arc::new(MetricsCollector::new()),
        })
    }
    
    /// Create new API server with in-memory database (for testing)
    pub async fn new_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let storage = StorageManager::new_in_memory()?;
        let python_executor = PythonExecutor::new()?;
        let nodejs_executor = NodeJSExecutor::new()?;
        
        Ok(Self {
            storage: Arc::new(storage),
            python_executor: Arc::new(Mutex::new(python_executor)),
            nodejs_executor: Arc::new(Mutex::new(nodejs_executor)),
            metrics: Arc::new(MetricsCollector::new()),
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
