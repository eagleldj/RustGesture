//! User interface module
//!
//! This module handles all UI components:
//! - System tray icon
//! - Settings window
//! - Notifications

pub mod tray;
pub mod config_dialog;

pub use tray::TrayIcon;
