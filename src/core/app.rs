//! Gesture application module
//!
//! This module provides a high-level application that integrates all components.

use crate::config::{config::GestureConfig, manager::ConfigManager};
use crate::core::{executor::CommandExecutor, gesture::Gesture, intent::GestureIntentFinder, recognizer::{create_shared_recognizer, GestureRecognizerEvent, SharedRecognizer}};
use std::sync::Mutex;
use tracing::{error, info, warn};

/// Gesture application
pub struct GestureApp {
    config_manager: Mutex<ConfigManager>,
    recognizer: SharedRecognizer,
    intent_finder: Mutex<GestureIntentFinder>,
    executor: CommandExecutor,
}

impl GestureApp {
    /// Create a new gesture application
    pub fn new() -> anyhow::Result<Self> {
        info!("Initializing RustGesture application...");

        // Load configuration
        let config_manager = Mutex::new(ConfigManager::new(None)?);
        let config = config_manager.lock().unwrap().config().clone();

        // Create intent finder
        let intent_finder = Mutex::new(GestureIntentFinder::new(config.clone()));

        // Create recognizer
        let recognizer = create_shared_recognizer(
            config.settings.clone(),
            config.settings.trigger_button.clone(),
        );

        // Create executor
        let executor = CommandExecutor::new();

        info!("RustGesture application initialized successfully");

        Ok(Self {
            config_manager,
            recognizer,
            intent_finder,
            executor,
        })
    }

    /// Start the application
    pub fn start(&self) -> anyhow::Result<()> {
        info!("Starting RustGesture...");

        // TODO: Install mouse hooks
        // TODO: Start event loop
        // TODO: Show tray icon

        info!("RustGesture started");
        Ok(())
    }

    /// Stop the application
    pub fn stop(&self) -> anyhow::Result<()> {
        info!("Stopping RustGesture...");

        // TODO: Uninstall hooks
        // TODO: Hide tray icon

        info!("RustGesture stopped");
        Ok(())
    }

    /// Handle a recognized gesture
    pub fn handle_gesture(&self, gesture: &Gesture) -> anyhow::Result<()> {
        let gesture_str = GestureIntentFinder::gesture_to_string(gesture);
        info!("Gesture recognized: {}", gesture_str);

        // Find matching intent
        let finder = self.intent_finder.lock().unwrap();
        if let Some(intent) = finder.find(gesture, None) {
            info!("Found matching action, executing...");

            // Execute the action
            if let Err(e) = self.executor.execute(&intent.action) {
                error!("Failed to execute action: {:?}", e);
            }
        } else {
            warn!("No matching action found for gesture: {}", gesture_str);
        }

        Ok(())
    }

    /// Reload configuration
    pub fn reload_config(&self) -> anyhow::Result<()> {
        info!("Reloading configuration...");

        let mut config_manager = self.config_manager.lock().unwrap();
        config_manager.reload()?;

        let config = config_manager.config().clone();

        // Update intent finder
        let mut finder = self.intent_finder.lock().unwrap();
        finder.update_config(config);

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Get the recognizer
    pub fn recognizer(&self) -> SharedRecognizer {
        self.recognizer.clone()
    }

    /// Get the config manager
    pub fn config_manager(&self) -> &Mutex<ConfigManager> {
        &self.config_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = GestureApp::new();
        assert!(app.is_ok(), "Failed to create GestureApp");
    }
}
