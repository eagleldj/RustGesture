//! Configuration data structures
//!
//! This module defines all the configuration structures used throughout the application.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Gesture direction (4-direction or 8-direction)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GestureDir {
    Up,
    Down,
    Left,
    Right,
    // Diagonal directions (only for first stroke in 8-direction mode)
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl GestureDir {
    /// Convert to display name
    pub fn as_str(&self) -> &'static str {
        match self {
            GestureDir::Up => "↑",
            GestureDir::Down => "↓",
            GestureDir::Left => "←",
            GestureDir::Right => "→",
            GestureDir::UpLeft => "↖",
            GestureDir::UpRight => "↗",
            GestureDir::DownLeft => "↙",
            GestureDir::DownRight => "↘",
        }
    }
}

/// Gesture modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GestureModifier {
    LeftButtonDown,
    RightButtonDown,
    MiddleButtonDown,
    WheelForward,
    WheelBackward,
}

/// A gesture consisting of a sequence of directions and optional modifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Gesture {
    pub directions: Vec<GestureDir>,
    pub modifiers: Vec<GestureModifier>,
}

impl Gesture {
    /// Create a new gesture from directions
    pub fn new(directions: Vec<GestureDir>) -> Self {
        Self {
            directions,
            modifiers: Vec::new(),
        }
    }

    /// Add a modifier to the gesture
    pub fn with_modifier(mut self, modifier: GestureModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    /// Check if gesture is empty
    pub fn is_empty(&self) -> bool {
        self.directions.is_empty()
    }

    /// Get the number of directions in the gesture
    pub fn len(&self) -> usize {
        self.directions.len()
    }
}

/// Action types that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    Keyboard(KeyboardAction),
    Mouse(MouseAction),
    Window(WindowAction),
    Run(RunAction),
}

impl Action {
    /// Get display info for this action (name and shortcut)
    pub fn display_info(&self) -> String {
        match self {
            Action::Keyboard(kb) => {
                format!("Keyboard: {}", kb.keys.join("+"))
            }
            Action::Mouse(mouse) => {
                let action_str = match mouse.action_type {
                    MouseActionType::Click => "Click",
                    MouseActionType::DoubleClick => "DoubleClick",
                };
                format!("Mouse: {} {}", mouse.button.as_str(), action_str)
            }
            Action::Window(win) => {
                format!("Window: {:?}", win.command)
            }
            Action::Run(run) => {
                format!("Run: {} {}", run.command, run.args.as_ref().unwrap_or(&"".to_string()))
            }
        }
    }
}

impl MouseButton {
    pub fn as_str(&self) -> &str {
        match self {
            MouseButton::Left => "Left",
            MouseButton::Right => "Right",
            MouseButton::Middle => "Middle",
            MouseButton::X1 => "X1",
            MouseButton::X2 => "X2",
        }
    }
}

/// Keyboard shortcut action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardAction {
    pub keys: Vec<String>, // Virtual key codes (e.g., "VK_CONTROL", "VK_C")
}

/// Mouse action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseAction {
    pub button: MouseButton,
    pub action_type: MouseActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseActionType {
    Click,
    DoubleClick,
}

/// Window command action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAction {
    pub command: WindowCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowCommand {
    Minimize,
    Maximize,
    Restore,
    Close,
    ShowDesktop,
}

/// Run program action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAction {
    pub command: String,
    pub args: Option<String>,
}

/// Trigger button for gestures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerButton {
    Right,
    Middle,
    X1,
    X2,
}

impl Default for TriggerButton {
    fn default() -> Self {
        TriggerButton::Middle
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Which mouse button triggers gestures
    pub trigger_button: TriggerButton,

    /// Minimum distance in pixels before gesture starts
    pub min_distance: u32,

    /// Minimum distance in pixels for a gesture direction
    pub effective_move: u32,

    /// Timeout in milliseconds before gesture is cancelled
    pub stay_timeout: u32,

    /// Enable 8-direction gestures (only first stroke)
    pub enable_8_direction: bool,

    /// Disable gestures in fullscreen applications
    pub disable_in_fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger_button: TriggerButton::Middle,
            min_distance: 5,
            effective_move: 20,
            stay_timeout: 500,
            enable_8_direction: false,
            disable_in_fullscreen: true,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureConfig {
    /// Configuration version
    pub version: u32,

    /// Global gesture mappings
    pub global_gestures: HashMap<String, Action>,

    /// Application-specific gesture mappings
    pub app_gestures: HashMap<String, HashMap<String, Action>>,

    /// Applications where gestures are disabled
    pub disabled_apps: HashSet<String>,

    /// Application settings
    pub settings: Settings,
}

impl Default for GestureConfig {
    fn default() -> Self {
        let mut global_gestures = HashMap::new();

        // Add some default gestures
        global_gestures.insert(
            "Right".to_string(),
            Action::Keyboard(KeyboardAction {
                keys: vec!["VK_CONTROL".to_string(), "VK_L".to_string()], // Lock? Or maybe Alt+Left
            }),
        );

        global_gestures.insert(
            "Down".to_string(),
            Action::Window(WindowAction {
                command: WindowCommand::Minimize,
            }),
        );

        global_gestures.insert(
            "Up".to_string(),
            Action::Window(WindowAction {
                command: WindowCommand::Maximize,
            }),
        );

        Self {
            version: 1,
            global_gestures,
            app_gestures: HashMap::new(),
            disabled_apps: HashSet::new(),
            settings: Settings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gesture_serialization() {
        let gesture = Gesture::new(vec![GestureDir::Right, GestureDir::Down]);
        let json = serde_json::to_string(&gesture).unwrap();
        assert!(json.contains("Right"));
        assert!(json.contains("Down"));
    }

    #[test]
    fn test_config_default() {
        let config = GestureConfig::default();
        assert_eq!(config.version, 1);
        assert!(!config.global_gestures.is_empty());
    }
}
