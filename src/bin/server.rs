//! NanoLambda Server
//!
//! Main entry point for the NanoLambda API server.

use anyhow::Result;
use nanolambda_api::ApiServer;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting NanoLambda Server v{}", env!("CARGO_PKG_VERSION"));
    info!("========================================");

    // Get database path from environment or use default
    let db_path =
        std::env::var("NANOLAMBDA_DB_PATH").unwrap_or_else(|_| "nanolambda.db".to_string());

    info!("Database path: {}", db_path);

    // Initialize API server
    info!("Initializing API server...");
    let api_server = ApiServer::new(&db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize API server: {}", e))?;

    info!("NanoLambda server started successfully!");
    info!("API endpoint: http://localhost:8080");
    info!("Health check: http://localhost:8080/health");

    // Start the server (this will block until shutdown)
    api_server
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
