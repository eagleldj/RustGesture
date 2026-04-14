//! Gesture recognizer module
//!
//! This module integrates the path tracker and gesture parsing to provide
//! high-level gesture recognition functionality.

use crate::config::config::Settings;
use crate::core::gesture::{Gesture, GestureDir, GestureModifier, Point};
use crate::core::parser::{calculate_4direction, calculate_8direction};
use crate::core::tracker::{PathTracker, TrackerEvent, TrackerState};
use crate::winapi::hook::MouseEvent;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Gesture recognizer that combines tracking and parsing
pub struct GestureRecognizer {
    tracker: PathTracker,
    max_gesture_steps: usize,
}

impl GestureRecognizer {
    /// Create a new gesture recognizer
    pub fn new(settings: Settings) -> Self {
        let tracker = PathTracker::new(settings);

        Self {
            tracker,
            max_gesture_steps: 12,
        }
    }

    /// Set the maximum gesture steps
    pub fn set_max_gesture_steps(&mut self, max: usize) {
        self.max_gesture_steps = max;
    }

    /// Set the event callback
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(GestureRecognizerEvent) + Send + 'static,
    {
        self.tracker.set_event_callback(move |event| {
            let recognizer_event = match event {
                TrackerEvent::GestureStarted(context) => {
                    GestureRecognizerEvent::GestureStarted(context)
                }
                TrackerEvent::GestureChanged(gesture) => {
                    GestureRecognizerEvent::GestureRecognized(gesture.clone(), false)
                }
                TrackerEvent::GestureCompleted(gesture) => {
                    GestureRecognizerEvent::GestureCompleted(gesture)
                }
                TrackerEvent::GestureCancelled => GestureRecognizerEvent::GestureCancelled,
                TrackerEvent::ModifierDetected(modifier) => {
                    GestureRecognizerEvent::ModifierDetected(modifier)
                }
                TrackerEvent::PositionUpdate(point) => {
                    GestureRecognizerEvent::PositionUpdate(point)
                }
            };
            callback(recognizer_event);
        });
    }

    /// Handle a mouse event
    ///
    /// All buttons can trigger gestures; the tracker determines the trigger
    /// from the event type and its own state.
    pub fn handle_mouse_event(&mut self, event: &MouseEvent) {
        self.tracker.handle_mouse_event(event);
    }

    /// Check for timeout
    pub fn check_timeout(&mut self) -> bool {
        self.tracker.check_timeout()
    }

    /// Get the current state
    pub fn state(&self) -> &TrackerState {
        self.tracker.state()
    }

    /// Get the current gesture
    pub fn current_gesture(&self) -> Option<&Gesture> {
        self.tracker.current_gesture()
    }

    /// Check if a gesture is being tracked
    pub fn is_tracking(&self) -> bool {
        matches!(self.state(), TrackerState::Tracking)
    }

    /// Check if in capture mode
    pub fn is_capturing(&self) -> bool {
        matches!(self.state(), TrackerState::Capturing)
    }
}

/// Gesture recognizer events
pub enum GestureRecognizerEvent {
    GestureStarted(crate::core::gesture::GestureContext),
    GestureRecognized(Gesture, bool), // gesture, is_final
    GestureCompleted(Gesture),
    GestureCancelled,
    ModifierDetected(GestureModifier),
    PositionUpdate(Point),
}

/// Shared gesture recognizer that can be used across threads
pub type SharedRecognizer = Arc<Mutex<GestureRecognizer>>;

/// Create a shared gesture recognizer
pub fn create_shared_recognizer(settings: Settings) -> SharedRecognizer {
    Arc::new(Mutex::new(GestureRecognizer::new(settings)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognizer_creation() {
        let settings = Settings::default();
        let recognizer = GestureRecognizer::new(settings);
        assert_eq!(recognizer.state(), &TrackerState::Idle);
    }

    #[test]
    fn test_recognizer_tracking() {
        let settings = Settings::default();
        let mut recognizer = GestureRecognizer::new(settings);

        // Simulate middle button down
        recognizer.handle_mouse_event(&MouseEvent::MiddleButtonDown(100, 100));
        assert!(recognizer.is_capturing());

        // Simulate movement
        recognizer.handle_mouse_event(&MouseEvent::MouseMove(120, 100));
        assert!(recognizer.is_tracking());
    }
}
