//! Gesture intent finder module
//!
//! This module matches recognized gestures to actions based on configuration.

use crate::config::config::{Action, GestureConfig, GestureEntry};
use crate::core::gesture::{Gesture, GestureModifier};
use std::collections::HashMap;
use tracing::{debug, info};

/// Gesture intent represents a matched gesture with its action
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GestureIntent {
    pub gesture: Gesture,
    pub action: Action,
    pub name: String,
}

#[allow(dead_code)]
impl GestureIntent {
    /// Check if this intent can be executed during the gesture (e.g., scroll wheel)
    pub fn can_execute_on_modifier(&self) -> bool {
        matches!(&self.action, Action::Window(_) | Action::Mouse(_))
    }
}

/// Gesture intent finder
#[allow(dead_code)]
pub struct GestureIntentFinder {
    config: GestureConfig,
    // Cache gesture strings for faster lookup
    global_cache: HashMap<String, GestureEntry>,
    // App-specific caches
    app_caches: HashMap<String, HashMap<String, GestureEntry>>,
}

#[allow(dead_code)]
impl GestureIntentFinder {
    /// Create a new gesture intent finder
    pub fn new(config: GestureConfig) -> Self {
        let global_cache = Self::build_global_cache(&config);
        let app_caches = Self::build_app_caches(&config);

        info!(
            "GestureIntentFinder created with {} global gestures and {} app-specific configs",
            global_cache.len(),
            app_caches.len()
        );

        Self {
            config,
            global_cache,
            app_caches,
        }
    }

    /// Build cache for global gestures
    fn build_global_cache(config: &GestureConfig) -> HashMap<String, GestureEntry> {
        let mut cache = HashMap::new();

        for (gesture_str, entry) in &config.global_gestures {
            cache.insert(gesture_str.clone(), entry.clone());
            debug!(
                "Cached global gesture: {} -> {:?}",
                gesture_str, entry.action
            );
        }

        cache
    }

    /// Build caches for app-specific gestures
    fn build_app_caches(config: &GestureConfig) -> HashMap<String, HashMap<String, GestureEntry>> {
        let mut app_caches = HashMap::new();

        for (app_name, gestures) in &config.app_gestures {
            let mut cache = HashMap::new();
            for (gesture_str, entry) in gestures {
                cache.insert(gesture_str.clone(), entry.clone());
                debug!(
                    "Cached app gesture: {} -> {} -> {:?}",
                    app_name, gesture_str, entry.action
                );
            }
            app_caches.insert(app_name.clone(), cache);
        }

        app_caches
    }

    /// Update configuration
    pub fn update_config(&mut self, config: GestureConfig) {
        self.global_cache = Self::build_global_cache(&config);
        self.app_caches = Self::build_app_caches(&config);
        self.config = config;
        info!("GestureIntentFinder configuration updated");
    }

    /// Find the intent for a gesture
    ///
    /// The lookup key includes the trigger button (e.g., "M_Right" for Middle button + Right direction).
    /// Only matches when both the trigger button and direction sequence match exactly.
    pub fn find(&self, gesture: &Gesture, app_name: Option<&str>) -> Option<GestureIntent> {
        let gesture_str = Self::gesture_to_string(gesture);
        debug!(
            "Looking up gesture: {} for app: {:?}",
            gesture_str, app_name
        );

        // Priority 1: App-specific gestures (with button prefix)
        if let Some(app) = app_name {
            if let Some(app_cache) = self.app_caches.get(app) {
                if let Some(entry) = app_cache.get(&gesture_str) {
                    debug!("Found app-specific gesture match for {}", app);
                    return Some(GestureIntent {
                        gesture: gesture.clone(),
                        action: entry.action.clone(),
                        name: entry.name.clone(),
                    });
                }
            }
        }

        // Priority 2: Global gestures (with button prefix)
        if let Some(entry) = self.global_cache.get(&gesture_str) {
            debug!("Found global gesture match (button-specific)");
            return Some(GestureIntent {
                gesture: gesture.clone(),
                action: entry.action.clone(),
                name: entry.name.clone(),
            });
        }

        debug!("No gesture match found");
        None
    }

    /// Find intent with modifiers (e.g., scroll wheel during gesture)
    pub fn find_with_modifiers(
        &self,
        gesture: &Gesture,
        app_name: Option<&str>,
    ) -> Option<GestureIntent> {
        // Try exact match first (with modifiers)
        let gesture_with_modifiers = Self::gesture_to_string_with_modifiers(gesture);
        debug!(
            "Looking up gesture with modifiers: {}",
            gesture_with_modifiers
        );

        // Check app-specific first
        if let Some(app) = app_name {
            if let Some(app_cache) = self.app_caches.get(app) {
                if let Some(entry) = app_cache.get(&gesture_with_modifiers) {
                    debug!(
                        "Found app-specific gesture match with modifiers for {}",
                        app
                    );
                    return Some(GestureIntent {
                        gesture: gesture.clone(),
                        action: entry.action.clone(),
                        name: entry.name.clone(),
                    });
                }
            }
        }

        // Check global
        if let Some(entry) = self.global_cache.get(&gesture_with_modifiers) {
            debug!("Found global gesture match with modifiers");
            return Some(GestureIntent {
                gesture: gesture.clone(),
                action: entry.action.clone(),
                name: entry.name.clone(),
            });
        }

        // Fall back to gesture without modifiers
        self.find(gesture, app_name)
    }

    /// Check if gesturing is enabled for the current context
    pub fn is_gesturing_enabled(&self, app_name: Option<&str>) -> bool {
        // Check if app is in disabled list
        if let Some(app) = app_name {
            if self.config.disabled_apps.contains(app) {
                debug!("Gesturing disabled for app: {}", app);
                return false;
            }
        }

        true
    }

    /// Convert gesture to string representation (includes trigger button prefix)
    /// Format: "M_Right" (Middle button + Right direction)
    pub fn gesture_to_string(gesture: &Gesture) -> String {
        let button_prefix = match gesture.trigger_button {
            crate::core::gesture::GestureTriggerButton::Right => "R_",
            crate::core::gesture::GestureTriggerButton::Middle => "M_",
            crate::core::gesture::GestureTriggerButton::X1 => "X1_",
            crate::core::gesture::GestureTriggerButton::X2 => "X2_",
        };
        format!(
            "{}{}",
            button_prefix,
            Self::gesture_directions_to_string(gesture)
        )
    }

    /// Convert gesture directions to string representation (direction-only, no button prefix)
    fn gesture_directions_to_string(gesture: &Gesture) -> String {
        gesture
            .directions
            .iter()
            .map(|dir| dir.dir_name())
            .collect::<Vec<_>>()
            .join(crate::core::gesture::GESTURE_DIR_SEPARATOR)
    }

    /// Convert gesture to string with modifiers (includes trigger button prefix)
    fn gesture_to_string_with_modifiers(gesture: &Gesture) -> String {
        let mut parts = Vec::new();

        // Add directions with button prefix
        parts.push(Self::gesture_to_string(gesture));

        // Add modifiers
        for modifier in &gesture.modifiers {
            let mod_str = match modifier {
                GestureModifier::LeftButtonDown => "+ LeftButton",
                GestureModifier::RightButtonDown => "+ RightButton",
                GestureModifier::MiddleButtonDown => "+ MiddleButton",
                GestureModifier::X1ButtonDown => "+ X1Button",
                GestureModifier::X2ButtonDown => "+ X2Button",
                GestureModifier::WheelForward => "+ WheelForward",
                GestureModifier::WheelBackward => "+ WheelBackward",
            };
            parts.push(mod_str.to_string());
        }

        parts.join(" ")
    }

    /// Get a gesture action from global or app-specific config
    pub fn get_action(&self, gesture_str: &str, app_name: Option<&str>) -> Option<&Action> {
        // Check app-specific first
        if let Some(app) = app_name {
            if let Some(app_cache) = self.app_caches.get(app) {
                if let Some(entry) = app_cache.get(gesture_str) {
                    return Some(&entry.action);
                }
            }
        }

        // Check global
        self.global_cache.get(gesture_str).map(|e| &e.action)
    }

    /// Get the configuration
    pub fn config(&self) -> &GestureConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::config::Settings;
    use serde_json;

    #[test]
    fn test_intent_finder_creation() {
        let config = GestureConfig::default();
        let finder = GestureIntentFinder::new(config);
        assert!(!finder.global_cache.is_empty());
    }

    #[test]
    fn test_gesture_to_string() {
        use crate::core::gesture::{GestureDir, GestureTriggerButton};

        let mut gesture = Gesture::new(GestureTriggerButton::Middle);
        gesture.add_direction(GestureDir::Right);
        gesture.add_direction(GestureDir::Down);

        let gesture_str = GestureIntentFinder::gesture_to_string(&gesture);
        assert_eq!(gesture_str, "M_Right → Down");
    }

    #[test]
    fn test_gesture_to_string_different_buttons() {
        use crate::core::gesture::{GestureDir, GestureTriggerButton};

        let mut gesture_r = Gesture::new(GestureTriggerButton::Right);
        gesture_r.add_direction(GestureDir::Right);
        assert_eq!(
            GestureIntentFinder::gesture_to_string(&gesture_r),
            "R_Right"
        );

        let mut gesture_x1 = Gesture::new(GestureTriggerButton::X1);
        gesture_x1.add_direction(GestureDir::Up);
        assert_eq!(GestureIntentFinder::gesture_to_string(&gesture_x1), "X1_Up");

        let mut gesture_x2 = Gesture::new(GestureTriggerButton::X2);
        gesture_x2.add_direction(GestureDir::Down);
        assert_eq!(
            GestureIntentFinder::gesture_to_string(&gesture_x2),
            "X2_Down"
        );
    }

    #[test]
    fn test_gesture_to_string_with_modifiers() {
        use crate::core::gesture::{GestureDir, GestureModifier, GestureTriggerButton};

        let mut gesture = Gesture::new(GestureTriggerButton::Middle);
        gesture.add_direction(GestureDir::Up);
        gesture.add_modifier(GestureModifier::WheelForward);

        let gesture_str = GestureIntentFinder::gesture_to_string_with_modifiers(&gesture);
        assert_eq!(gesture_str, "M_Up + WheelForward");
    }

    #[test]
    fn test_find_with_button_matching() {
        use crate::core::gesture::{GestureDir, GestureTriggerButton};

        let config = GestureConfig::default();
        let finder = GestureIntentFinder::new(config);

        // Default config has "M_Right" for Middle button
        let mut gesture_m = Gesture::new(GestureTriggerButton::Middle);
        gesture_m.add_direction(GestureDir::Right);
        assert!(finder.find(&gesture_m, None).is_some());

        // Right button with same direction should not match button-specific entry
        // but might match direction-only fallback if present
        let mut gesture_r = Gesture::new(GestureTriggerButton::Right);
        gesture_r.add_direction(GestureDir::Right);
        // No "R_Right" in config, but might fall back to "Right"
        let result = finder.find(&gesture_r, None);
        // Depending on backward compat, this could match or not
        // Since default config only has "M_" prefixed keys, no "Right" fallback exists
        // So this should NOT match
        assert!(
            result.is_none(),
            "Right button gesture should not match Middle button config"
        );
    }

    #[test]
    fn test_disabled_apps() {
        use crate::core::gesture::{GestureDir, GestureTriggerButton};

        let config = GestureConfig::default();
        let mut disabled_apps = std::collections::HashSet::new();
        disabled_apps.insert("notepad.exe".to_string());
        let config = GestureConfig {
            disabled_apps,
            ..config
        };

        let finder = GestureIntentFinder::new(config);
        assert!(!finder.is_gesturing_enabled(Some("notepad.exe")));
        assert!(finder.is_gesturing_enabled(Some("chrome.exe")));
    }
}
