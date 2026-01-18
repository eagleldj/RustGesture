//! Gesture application module
//!
//! This module provides a high-level application that integrates all components.

use crate::config::{config::GestureConfig, manager::ConfigManager};
use crate::core::{executor::CommandExecutor, gesture::Gesture, intent::GestureIntentFinder, recognizer::{create_shared_recognizer, GestureRecognizerEvent, SharedRecognizer}};
use crate::core::hook_callback::GestureHookCallback;
use crate::winapi::hook::MouseHook;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, error, info, warn};

/// Gesture application
pub struct GestureApp {
    config_manager: Mutex<ConfigManager>,
    recognizer: SharedRecognizer,
    intent_finder: Mutex<GestureIntentFinder>,
    executor: CommandExecutor,
    hook: Option<MouseHook>,
    enabled: Arc<AtomicBool>,
}

impl GestureApp {
    /// Create a new gesture application
    pub fn new() -> anyhow::Result<Self> {
        info!("Initializing RustGesture application...");

        // Load configuration
        let config_manager = Mutex::new(ConfigManager::new(None)?);
        let config = config_manager.lock().unwrap().config().clone();

        // Create enabled state
        let enabled = Arc::new(AtomicBool::new(true));

        // Create intent finder
        let intent_finder = Mutex::new(GestureIntentFinder::new(config.clone()));

        // Create recognizer
        let recognizer = create_shared_recognizer(
            config.settings.clone(),
            config.settings.trigger_button.clone(),
        );

        // Create executor
        let executor = CommandExecutor::new();

        // Set up gesture recognition callback
        let intent_finder_clone = Arc::new(Mutex::new(GestureIntentFinder::new(config.clone())));
        let executor_clone = executor.clone(); // Executor needs to be cloned

        // Set event callback on recognizer
        {
            let mut recognizer = recognizer.lock().unwrap();
            recognizer.set_event_callback(move |event| {
                match event {
                    GestureRecognizerEvent::GestureCompleted(gesture) => {
                        info!("Gesture completed: {:?}", gesture);

                        // Find matching intent
                        let finder = intent_finder_clone.lock().unwrap();
                        if let Some(intent) = finder.find(&gesture, None) {
                            info!("Found matching action for gesture: {:?}", gesture);

                            // Execute the action
                            if let Err(e) = executor_clone.execute(&intent.action) {
                                error!("Failed to execute action: {:?}", e);
                            }
                        } else {
                            warn!("No matching action found for gesture: {:?}", gesture);
                        }
                    }
                    GestureRecognizerEvent::GestureCancelled => {
                        debug!("Gesture cancelled");
                    }
                    GestureRecognizerEvent::GestureStarted(context) => {
                        debug!("Gesture started at: ({}, {})", context.start_point.x, context.start_point.y);
                    }
                    GestureRecognizerEvent::GestureRecognized(gesture, _is_final) => {
                        debug!("Gesture recognized: {:?}", gesture);
                    }
                    GestureRecognizerEvent::ModifierDetected(modifier) => {
                        debug!("Modifier detected: {:?}", modifier);
                    }
                }
            });
        }

        // Create intent finder for the app struct
        let intent_finder = Mutex::new(GestureIntentFinder::new(config.clone()));

        // Create and install mouse hook
        let mut hook = MouseHook::new();
        let callback = GestureHookCallback::new(
            recognizer.clone(),
            enabled.clone(),
        );
        hook.set_callback(Box::new(callback));

        // Install the hook
        hook.install()?;

        info!("RustGesture application initialized successfully");

        Ok(Self {
            config_manager,
            recognizer,
            intent_finder,
            executor,
            hook: Some(hook),
            enabled,
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

impl Drop for GestureApp {
    fn drop(&mut self) {
        info!("GestureApp dropping - cleaning up resources");

        // Uninstall the hook if it exists
        if let Some(mut hook) = self.hook.take() {
            if let Err(e) = hook.uninstall() {
                error!("Failed to uninstall mouse hook: {:?}", e);
            } else {
                info!("Mouse hook uninstalled successfully");
            }
        }

        info!("GestureApp cleanup complete");
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
