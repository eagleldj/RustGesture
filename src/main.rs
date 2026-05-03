mod config;
mod core;
mod ui;
mod winapi;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // DPI awareness: WH_MOUSE_LL always reports physical pixel coordinates.
    // Use per-monitor DPI awareness (v2) so coordinates are consistent across
    // multiple monitors with different DPI scaling.
    unsafe {
        use windows::core::PCSTR;
        use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        use windows::Win32::Foundation::*;

        // Try SetProcessDpiAwareness(2) from shcore.dll first (per-monitor DPI aware)
        let shcore_name: Vec<u16> = "shcore\0".encode_utf16().collect();
        let shcore = windows::Win32::System::LibraryLoader::GetModuleHandleW(
            windows::core::PCWSTR::from_raw(shcore_name.as_ptr()),
        )
        .unwrap_or_default();

        let mut dpi_awareness_set = false;
        if !shcore.is_invalid() && shcore != HMODULE::default() {
            let proc_name: Vec<u8> = b"SetProcessDpiAwareness\0".to_vec();
            let proc_addr = GetProcAddress(shcore, PCSTR::from_raw(proc_name.as_ptr()));
            if let Some(proc) = proc_addr {
                let func: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(proc);
                let result = func(2); // PROCESS_PER_MONITOR_DPI_AWARE
                info!("SetProcessDpiAwareness(2) called, result={}", result);
                if result >= 0 {
                    dpi_awareness_set = true;
                }
            }
        }

        // Fallback to SetProcessDPIAware from user32.dll
        if !dpi_awareness_set {
            let user32 = GetModuleHandleW(windows::core::w!("user32")).unwrap_or_default();
            if !user32.is_invalid() && user32 != HMODULE::default() {
                let proc_name: Vec<u8> = b"SetProcessDPIAware\0".to_vec();
                let proc_addr = GetProcAddress(user32, PCSTR::from_raw(proc_name.as_ptr()));
                if let Some(proc) = proc_addr {
                    let func: unsafe extern "system" fn() -> i32 = std::mem::transmute(proc);
                    let result = func();
                    info!("SetProcessDPIAware called (fallback), result={}", result);
                }
            }
        }
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("RustGesture v0.1.0 starting...");

    // Enabled state (can be toggled from tray)
    let enabled = Arc::new(AtomicBool::new(true));

    // Get config directory
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustGesture");

    let config_path = config_dir.join("config.json");

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

    // Channel to receive tray icon creation result and shutdown signal
    let (tray_tx, tray_rx) = std::sync::mpsc::channel();
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();

    let config_path_clone = config_path.clone();
    let _message_loop_handle = std::thread::spawn(move || {
        if let (Some(recognizer), Some(_config), Some(enabled)) =
            (recognizer_opt, config_opt, enabled_opt)
        {
            // Create system tray IN MESSAGE LOOP THREAD
            info!("Initializing system tray in message loop thread...");
            let tray = match ui::TrayIcon::new(enabled.clone(), shutdown_tx, config_path_clone) {
                Ok(t) => {
                    info!("System tray created successfully in message loop thread");
                    let _ = tray_tx.send(Ok(()));
                    Some(t)
                }
                Err(e) => {
                    tracing::error!("Failed to create system tray: {:?}", e);
                    let _ = tray_tx.send(Err(e));
                    None
                }
            };

            // Create and install hook IN THIS THREAD
            use crate::core::hook_callback::GestureHookCallback;
            use crate::winapi::hook::MouseHook;

            let mut hook = MouseHook::new();
            let callback = GestureHookCallback::new(recognizer, enabled);
            hook.set_callback(Box::new(callback));

            // Install hook in message loop thread
            unsafe {
                use windows::Win32::Foundation::HWND;
                use windows::Win32::UI::WindowsAndMessaging::*;

                if let Err(e) = hook.install() {
                    tracing::error!(
                        "Failed to install mouse hook in message loop thread: {:?}",
                        e
                    );
                } else {
                    tracing::info!("Mouse hook installed in message loop thread");
                }

                let mut msg = MSG::default();
                // Message loop - this is required for hooks to work
                // Keep tray alive in this scope so it doesn't get dropped
                let _tray = tray;
                while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    });

    info!("Windows message loop started");

    // Wait for tray icon to be created
    tray_rx.recv()??;
    info!("System tray initialized");

    info!("RustGesture started successfully");
    info!(
        "Gesture recognition is {}",
        if enabled.load(Ordering::SeqCst) {
            "enabled"
        } else {
            "disabled"
        }
    );
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
        _ = async {
            // Wait for shutdown signal from tray menu
            let _ = shutdown_rx.recv();
        } => {
            info!("Received exit request from tray menu, shutting down...");
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
