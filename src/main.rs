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
use std::thread;

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

    // CRITICAL: Start Windows message loop in a separate thread
    // Low-level mouse hooks (WH_MOUSE_LL) MUST be installed in the thread that runs the message loop!
    info!("Starting Windows message loop...");

    // Get references to what we need in the message loop thread
    let recognizer_opt = gesture_app.as_ref().map(|app| app.recognizer());
    let config_opt = gesture_app.as_ref().map(|app| app.config());
    let enabled_opt = gesture_app.as_ref().map(|app| app.enabled.clone());

    let _message_loop_handle = std::thread::spawn(move || {
        if let (Some(recognizer), Some(config), Some(enabled)) = (recognizer_opt, config_opt, enabled_opt) {
            // Create and install hook IN THIS THREAD
            use crate::winapi::hook::MouseHook;
            use crate::core::hook_callback::GestureHookCallback;

            let mut hook = MouseHook::new();
            let callback = GestureHookCallback::new(
                recognizer,
                config.settings.trigger_button.clone(),
                enabled,
            );
            hook.set_callback(Box::new(callback));

            // Install hook in message loop thread
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::*;
                use windows::Win32::Foundation::HWND;

                if let Err(e) = hook.install() {
                    tracing::error!("Failed to install mouse hook in message loop thread: {:?}", e);
                } else {
                    tracing::info!("Mouse hook installed in message loop thread");
                }

                let mut msg = MSG::default();
                // Message loop - this is required for hooks to work
                while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    });

    info!("Windows message loop started");

    // Initialize system tray
    info!("Initializing system tray...");
    let _tray = ui::TrayIcon::new(enabled.clone())?;
    info!("System tray initialized");

    info!("RustGesture started successfully");
    info!("Gesture recognition is {}", if enabled.load(Ordering::SeqCst) { "enabled" } else { "disabled" });
    info!("Multi-button support enabled: Right, Middle, X1, X2");

    // Start performance monitoring task
    let enabled_monitor = enabled.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if !enabled_monitor.load(Ordering::SeqCst) {
                break;
            }

            // Get hook performance stats
            // let (call_count, last_duration_ns) = winapi::hook::get_hook_stats();
            // let last_duration_us = last_duration_ns / 1000; // Convert to microseconds

            // tracing::info!(
            //     "Hook Stats: calls={}, last_duration={}μs ({}ns)",
            //     call_count,
            //     last_duration_us,
            //     last_duration_ns
            // );

            // Alert if hook is taking too long
            // if last_duration_ns > 100_000 { // > 100 microseconds
            //     tracing::warn!(
            //         "⚠️  HOOK LATENCY HIGH: {}μs - This will cause mouse lag!",
            //         last_duration_us
            //     );
            // } else if last_duration_ns > 10_000 { // > 10 microseconds
            //     tracing::warn!(
            //         "⚠️  Hook latency elevated: {}μs",
            //         last_duration_us
            //     );
            // }
        }
    });

    // Keep the application running
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
    }

    info!("Shutting down...");

    // Cleanup gesture app if it exists
    if let Some(app) = gesture_app {
        drop(app);
        info!("Gesture application cleaned up");
    }

    // Note: message loop thread will be terminated when process exits

    Ok(())
}
