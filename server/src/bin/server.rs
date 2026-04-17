//! NanoLambda Server
//!
//! Main entry point for the NanoLambda API server.

use anyhow::Result;
use nanolambda_api::ApiServer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with RUST_LOG support
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,nanolambda=debug")),
        )
        .init();

    info!("Starting NanoLambda Server v{}", env!("CARGO_PKG_VERSION"));
    info!("========================================");

    // Get configuration from environment
    let db_path =
        std::env::var("NANOLAMBDA_DB_PATH").unwrap_or_else(|_| "nanolambda.db".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    info!("Database path: {}", db_path);

    // Initialize API server
    info!("Initializing API server...");
    let api_server = ApiServer::new(&db_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize API server: {}", e))?;

    info!("NanoLambda server started successfully!");
    info!("API endpoint: http://{}:{}", host, port);
    info!("Health check: http://{}:{}/health", host, port);

    // Start the server with graceful shutdown (this will block until shutdown)
    api_server
        .run_with_shutdown(&host, port)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    info!("Server shut down gracefully");
    Ok(())
}
