//! Gesture recognition core structures
//!
//! This module defines the core gesture data structures used during recognition.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Mouse button that can trigger gestures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GestureTriggerButton {
    Right,
    Middle,
    X1,
    X2,
}

impl fmt::Display for GestureTriggerButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GestureTriggerButton::Right => write!(f, "Right"),
            GestureTriggerButton::Middle => write!(f, "Middle"),
            GestureTriggerButton::X1 => write!(f, "X1"),
            GestureTriggerButton::X2 => write!(f, "X2"),
        }
    }
}

/// Gesture direction (cardinal and diagonal)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GestureDir {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl GestureDir {
    /// Check if this is a diagonal direction
    pub fn is_diagonal(&self) -> bool {
        matches!(
            self,
            GestureDir::UpLeft | GestureDir::UpRight | GestureDir::DownLeft | GestureDir::DownRight
        )
    }

    /// Convert to 4-direction equivalent
    pub fn to_cardinal(&self) -> GestureDir {
        match self {
            GestureDir::UpLeft | GestureDir::UpRight => GestureDir::Up,
            GestureDir::DownLeft | GestureDir::DownRight => GestureDir::Down,
            _ => *self,
        }
    }
}

/// Gesture modifier detected during gesture tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GestureModifier {
    LeftButtonDown,
    RightButtonDown,
    MiddleButtonDown,
    WheelForward,
    WheelBackward,
}

impl GestureModifier {
    /// Check if this modifier is a scroll wheel modifier
    pub fn is_scroll(&self) -> bool {
        matches!(self, GestureModifier::WheelForward | GestureModifier::WheelBackward)
    }
}

/// A gesture being tracked
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gesture {
    /// Trigger button that started this gesture
    pub trigger_button: GestureTriggerButton,

    /// Sequence of directions
    pub directions: Vec<GestureDir>,

    /// Modifiers detected during gesture
    pub modifiers: Vec<GestureModifier>,
}

impl Gesture {
    /// Create a new gesture
    pub fn new(trigger_button: GestureTriggerButton) -> Self {
        Self {
            trigger_button,
            directions: Vec::new(),
            modifiers: Vec::new(),
        }
    }

    /// Add a direction to the gesture
    pub fn add_direction(&mut self, dir: GestureDir) {
        // Only add if different from last direction
        if self.directions.last() != Some(&dir) {
            self.directions.push(dir);
        }
    }

    /// Add a modifier to the gesture
    pub fn add_modifier(&mut self, modifier: GestureModifier) {
        if !self.modifiers.contains(&modifier) {
            self.modifiers.push(modifier);
        }
    }

    /// Check if gesture is empty
    pub fn is_empty(&self) -> bool {
        self.directions.is_empty()
    }

    /// Get the number of directions
    pub fn len(&self) -> usize {
        self.directions.len()
    }

    /// Get the last direction
    pub fn last(&self) -> Option<&GestureDir> {
        self.directions.last()
    }

    /// Create a display string for the gesture
    pub fn display_string(&self) -> String {
        self.directions
            .iter()
            .map(|dir| match dir {
                GestureDir::Up => "↑",
                GestureDir::Down => "↓",
                GestureDir::Left => "←",
                GestureDir::Right => "→",
                GestureDir::UpLeft => "↖",
                GestureDir::UpRight => "↗",
                GestureDir::DownLeft => "↙",
                GestureDir::DownRight => "↘",
            })
            .collect::<Vec<_>>()
            .join(" → ")
    }
}

impl fmt::Display for Gesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_string())
    }
}

/// Point in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculate distance to another point
    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calculate vector from this point to another
    pub fn vector_to(&self, other: &Point) -> Point {
        Point {
            x: other.x - self.x,
            y: other.y - self.y,
        }
    }
}

/// Gesture context information
#[derive(Debug, Clone)]
pub struct GestureContext {
    /// Starting point of the gesture
    pub start_point: Point,

    /// Current point
    pub current_point: Point,

    /// Active window handle
    pub window_handle: Option<isize>,

    /// Process ID of active application
    pub process_id: Option<u32>,

    /// Is the application in fullscreen mode?
    pub is_fullscreen: bool,
}

impl GestureContext {
    pub fn new(start_point: Point) -> Self {
        Self {
            start_point,
            current_point: start_point,
            window_handle: None,
            process_id: None,
            is_fullscreen: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gesture_creation() {
        let mut gesture = Gesture::new(GestureTriggerButton::Middle);
        gesture.add_direction(GestureDir::Right);
        gesture.add_direction(GestureDir::Down);

        assert_eq!(gesture.len(), 2);
        assert!(!gesture.is_empty());
    }

    #[test]
    fn test_gesture_no_duplicate_directions() {
        let mut gesture = Gesture::new(GestureTriggerButton::Middle);
        gesture.add_direction(GestureDir::Right);
        gesture.add_direction(GestureDir::Right); // Duplicate

        assert_eq!(gesture.len(), 1);
    }

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0, 0);
        let p2 = Point::new(3, 4);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }

    #[test]
    fn test_gesture_display() {
        let mut gesture = Gesture::new(GestureTriggerButton::Middle);
        gesture.add_direction(GestureDir::Right);
        gesture.add_direction(GestureDir::Down);

        assert_eq!(gesture.display_string(), "→ → ↓");
    }
}
