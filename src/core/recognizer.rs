//! Gesture recognizer module
//!
//! This module integrates the path tracker and gesture parsing to provide
//! high-level gesture recognition functionality.

use crate::config::config::{Settings, TriggerButton};
use crate::core::gesture::{Gesture, GestureDir, GestureModifier, GestureTriggerButton};
use crate::core::parser::{calculate_4direction, calculate_8direction};
use crate::core::tracker::{PathTracker, TrackerEvent, TrackerState};
use crate::winapi::hook::MouseEvent;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Gesture recognizer that combines tracking and parsing
pub struct GestureRecognizer {
    tracker: PathTracker,
    trigger_button: GestureTriggerButton,
    max_gesture_steps: usize,
}

impl GestureRecognizer {
    /// Create a new gesture recognizer
    pub fn new(settings: Settings, trigger_button: TriggerButton) -> Self {
        let tracker = PathTracker::new(settings);

        Self {
            tracker,
            trigger_button: Self::convert_trigger_button(trigger_button),
            max_gesture_steps: 12,
        }
    }

    /// Convert config trigger button to core trigger button
    fn convert_trigger_button(btn: TriggerButton) -> GestureTriggerButton {
        match btn {
            TriggerButton::Right => GestureTriggerButton::Right,
            TriggerButton::Middle => GestureTriggerButton::Middle,
            TriggerButton::X1 => GestureTriggerButton::X1,
            TriggerButton::X2 => GestureTriggerButton::X2,
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
                TrackerEvent::GestureCancelled => {
                    GestureRecognizerEvent::GestureCancelled
                }
                TrackerEvent::ModifierDetected(modifier) => {
                    GestureRecognizerEvent::ModifierDetected(modifier)
                }
            };
            callback(recognizer_event);
        });
    }

    /// Handle a mouse event
    pub fn handle_mouse_event(&mut self, event: &MouseEvent) {
        // Determine trigger button from event type for multi-button support
        let trigger_btn = match event {
            MouseEvent::RightButtonDown(_, _) => GestureTriggerButton::Right,
            MouseEvent::MiddleButtonDown(_, _) => GestureTriggerButton::Middle,
            MouseEvent::XButtonDown(_, _, btn) => {
                if *btn == 1 { GestureTriggerButton::X1 } else { GestureTriggerButton::X2 }
            }
            _ => self.trigger_button, // Use configured trigger for other events
        };

        self.tracker.handle_mouse_event(event, trigger_btn);
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
}

/// Shared gesture recognizer that can be used across threads
pub type SharedRecognizer = Arc<Mutex<GestureRecognizer>>;

/// Create a shared gesture recognizer
pub fn create_shared_recognizer(
    settings: Settings,
    trigger_button: TriggerButton,
) -> SharedRecognizer {
    Arc::new(Mutex::new(GestureRecognizer::new(settings, trigger_button)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognizer_creation() {
        let settings = Settings::default();
        let recognizer = GestureRecognizer::new(settings, TriggerButton::Middle);
        assert_eq!(recognizer.state(), &TrackerState::Idle);
    }

    #[test]
    fn test_recognizer_tracking() {
        let settings = Settings::default();
        let mut recognizer = GestureRecognizer::new(settings, TriggerButton::Middle);

        // Simulate middle button down
        recognizer.handle_mouse_event(&MouseEvent::MiddleButtonDown(100, 100));
        assert!(recognizer.is_capturing());

        // Simulate movement
        recognizer.handle_mouse_event(&MouseEvent::MouseMove(120, 100));
        assert!(recognizer.is_tracking());
    }
}
