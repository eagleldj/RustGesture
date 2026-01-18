mod config;
mod core;
mod winapi;

use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    info!("RustGesture v0.1.0 starting...");

    // TODO: Initialize configuration manager
    // TODO: Install mouse hooks
    // TODO: Start gesture recognition
    // TODO: Initialize system tray

    info!("RustGesture started successfully");

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}
