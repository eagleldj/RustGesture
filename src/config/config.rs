//! Configuration data structures
//!
//! This module defines all the configuration structures used throughout the application.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Gesture direction (4-direction or 8-direction)
#[allow(dead_code)]
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

#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GestureModifier {
    LeftButtonDown,
    RightButtonDown,
    MiddleButtonDown,
    X1ButtonDown,
    X2ButtonDown,
    WheelForward,
    WheelBackward,
}

/// A gesture consisting of a sequence of directions and optional modifiers
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Gesture {
    pub directions: Vec<GestureDir>,
    pub modifiers: Vec<GestureModifier>,
}

#[allow(dead_code)]
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
                format!(
                    "Run: {} {}",
                    run.command,
                    run.args.as_ref().unwrap_or(&"".to_string())
                )
            }
        }
    }
}

/// A gesture entry combining a user-friendly name with an action.
/// Stored as the value in gesture HashMaps (key is the direction sequence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureEntry {
    #[serde(default)]
    pub name: String,
    #[serde(flatten)]
    pub action: Action,
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

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
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

    /// Show gesture trail overlay
    pub show_trail: bool,

    /// Show gesture name tooltip after recognition
    pub show_gesture_name: bool,

    /// Trail line width in pixels
    pub trail_width: u32,

    /// Trail color for right-button gesture (hex, e.g. "#0096FF")
    pub trail_color_right: String,

    /// Trail color for middle-button gesture (hex, e.g. "#00CC66")
    pub trail_color_middle: String,

    /// Trail color for X-button gesture (hex, e.g. "#FF8800")
    pub trail_color_x: String,

    /// Trail color for unrecognized gesture (hex, e.g. "#6633CC")
    pub trail_color_unknown: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_distance: 5,
            effective_move: 20,
            stay_timeout: 500,
            enable_8_direction: false,
            disable_in_fullscreen: true,
            show_trail: true,
            show_gesture_name: true,
            trail_width: 3,
            trail_color_right: "#0096FF".to_string(),
            trail_color_middle: "#00CC66".to_string(),
            trail_color_x: "#FF8800".to_string(),
            trail_color_unknown: "#6633CC".to_string(),
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureConfig {
    /// Configuration version
    pub version: u32,

    /// Global gesture mappings
    pub global_gestures: HashMap<String, GestureEntry>,

    /// Application-specific gesture mappings
    pub app_gestures: HashMap<String, HashMap<String, GestureEntry>>,

    /// Applications where gestures are disabled
    pub disabled_apps: HashSet<String>,

    /// Application settings
    pub settings: Settings,
}

impl Default for GestureConfig {
    fn default() -> Self {
        let mut global_gestures = HashMap::new();

        // Default gestures use button prefix: M_ = Middle, R_ = Right, X1_ = X1, X2_ = X2
        global_gestures.insert(
            "M_Right".to_string(),
            GestureEntry {
                name: "Ctrl+L".to_string(),
                action: Action::Keyboard(KeyboardAction {
                    keys: vec!["VK_CONTROL".to_string(), "VK_L".to_string()],
                }),
            },
        );

        global_gestures.insert(
            "M_Down".to_string(),
            GestureEntry {
                name: "最小化".to_string(),
                action: Action::Window(WindowAction {
                    command: WindowCommand::Minimize,
                }),
            },
        );

        global_gestures.insert(
            "M_Up".to_string(),
            GestureEntry {
                name: "最大化".to_string(),
                action: Action::Window(WindowAction {
                    command: WindowCommand::Maximize,
                }),
            },
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
    fn test_gesture_entry_serialization() {
        let entry = GestureEntry {
            name: "复制".to_string(),
            action: Action::Keyboard(KeyboardAction {
                keys: vec!["VK_CONTROL".to_string(), "VK_C".to_string()],
            }),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"name\":\"复制\""));
        assert!(json.contains("\"type\":\"keyboard\""));
        assert!(json.contains("\"keys\""));

        // Round-trip
        let deserialized: GestureEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "复制");
    }

    #[test]
    fn test_gesture_entry_backward_compatible() {
        // Old format without name field should deserialize with empty name
        let json = r#"{"type":"window","command":"Maximize"}"#;
        let entry: GestureEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name, "");
        assert!(matches!(entry.action, Action::Window(_)));
    }

    #[test]
    fn test_config_default() {
        let config = GestureConfig::default();
        assert_eq!(config.version, 1);
        assert!(!config.global_gestures.is_empty());
    }
}
