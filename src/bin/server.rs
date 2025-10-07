//! NanoLambda Server
//!
//! Main entry point for the NanoLambda API server.

use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting NanoLambda Server v{}", nanolambda::VERSION);
    info!("========================================");
    
    // TODO: Initialize components
    info!("Initializing VMM...");
    info!("Initializing API server...");
    info!("Initializing scheduler...");
    
    info!("NanoLambda server started successfully!");
    info!("API endpoint: http://localhost:8080");
    info!("Metrics endpoint: http://localhost:9090/metrics");
    
    // Keep server running
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down gracefully...");
    
    Ok(())
}
