mod config;
mod core;
mod winapi;
mod ui;

use anyhow::Result;
use tracing::{info, error, warn, debug};
use tracing_subscriber;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;

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

    // Enabled state (can be toggled from tray)
    let enabled = Arc::new(AtomicBool::new(true));

    // Get config directory
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rustgesture");

    info!("Config directory: {:?}", config_dir);

    // Create gesture app (integrates all components)
    info!("Initializing gesture application...");
    let gesture_app = match core::app::GestureApp::new() {
        Ok(app) => {
            info!("Gesture application initialized");
            Some(app)
        }
        Err(e) => {
            error!("Failed to initialize gesture application: {}", e);
            error!("Gesture recognition will not be available");
            None
        }
    };

    // Initialize system tray
    info!("Initializing system tray...");
    let _tray = ui::TrayIcon::new(enabled.clone())?;
    info!("System tray initialized");

    info!("RustGesture started successfully");
    info!("Gesture recognition is {}", if enabled.load(Ordering::SeqCst) { "enabled" } else { "disabled" });

    // Keep the application running
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received termination signal, shutting down...");
        }
    }

    info!("Shutting down...");

    // Cleanup gesture app if it exists
    if let Some(app) = gesture_app {
        drop(app);
        info!("Gesture application cleaned up");
    }

    Ok(())
}
