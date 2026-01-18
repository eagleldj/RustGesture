//! Path tracker module
//!
//! This module tracks mouse movements and determines when to start gesture recognition.

use crate::core::gesture::{Gesture, GestureContext, GestureDir, GestureModifier, GestureTriggerButton, Point};
use crate::config::config::Settings;
use crate::winapi::hook::MouseEvent;
use tracing::{debug, trace};
use std::time::{Duration, Instant};

/// Path tracker state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerState {
    Idle,
    Capturing,
    Tracking,
}

/// Path tracker events
pub enum TrackerEvent {
    GestureStarted(GestureContext),
    GestureChanged(Gesture),
    GestureCompleted(Gesture),
    GestureCancelled,
    ModifierDetected(GestureModifier),
}

/// Path tracker
pub struct PathTracker {
    state: TrackerState,
    settings: Settings,

    // Tracking state
    start_point: Option<Point>,
    last_point: Option<Point>,
    last_effective_point: Option<Point>,
    current_gesture: Option<Gesture>,

    // Timing
    mouse_down_time: Option<Instant>,
    last_move_time: Option<Instant>,

    // Event callback
    event_callback: Option<Box<dyn Fn(TrackerEvent) + Send>>,
}

impl PathTracker {
    /// Create a new path tracker
    pub fn new(settings: Settings) -> Self {
        Self {
            state: TrackerState::Idle,
            settings,
            start_point: None,
            last_point: None,
            last_effective_point: None,
            current_gesture: None,
            mouse_down_time: None,
            last_move_time: None,
            event_callback: None,
        }
    }

    /// Set the event callback
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(TrackerEvent) + Send + 'static,
    {
        self.event_callback = Some(Box::new(callback));
    }

    /// Handle a mouse event
    pub fn handle_mouse_event(&mut self, event: &MouseEvent, trigger_button: GestureTriggerButton) {
        match event {
            MouseEvent::MouseMove(x, y) => self.on_mouse_move(*x, *y),
            MouseEvent::LeftButtonDown(x, y) => {
                if trigger_button == GestureTriggerButton::Right
                    || trigger_button == GestureTriggerButton::Middle
                {
                    self.on_modifier(GestureModifier::LeftButtonDown);
                } else {
                    self.on_mouse_down(*x, *y, trigger_button);
                }
            }
            MouseEvent::RightButtonDown(x, y) => {
                if trigger_button == GestureTriggerButton::Right {
                    self.on_mouse_down(*x, *y, trigger_button);
                } else {
                    self.on_modifier(GestureModifier::RightButtonDown);
                }
            }
            MouseEvent::MiddleButtonDown(x, y) => {
                if trigger_button == GestureTriggerButton::Middle {
                    self.on_mouse_down(*x, *y, trigger_button);
                } else {
                    self.on_modifier(GestureModifier::MiddleButtonDown);
                }
            }
            MouseEvent::RightButtonUp(_, _) | MouseEvent::MiddleButtonUp(_, _) => {
                if self.state == TrackerState::Tracking {
                    self.on_mouse_up();
                }
            }
            MouseEvent::MouseWheel(_, _, delta) => {
                let modifier = if *delta > 0 {
                    GestureModifier::WheelForward
                } else {
                    GestureModifier::WheelBackward
                };
                self.on_modifier(modifier);
            }
            _ => {}
        }
    }

    /// Handle mouse button down
    fn on_mouse_down(&mut self, x: i32, y: i32, button: GestureTriggerButton) {
        debug!("Mouse down: ({}, {}), button: {:?}", x, y, button);

        self.state = TrackerState::Capturing;
        self.start_point = Some(Point::new(x, y));
        self.last_point = Some(Point::new(x, y));
        self.last_effective_point = Some(Point::new(x, y));
        self.mouse_down_time = Some(Instant::now());
        self.last_move_time = Some(Instant::now());

        // Create new gesture
        self.current_gesture = Some(Gesture::new(button));
    }

    /// Handle mouse move
    fn on_mouse_move(&mut self, x: i32, y: i32) {
        if self.state != TrackerState::Capturing && self.state != TrackerState::Tracking {
            return;
        }

        let current_point = Point::new(x, y);
        let start = self.start_point.unwrap();

        // Check initial movement threshold
        if self.state == TrackerState::Capturing {
            let distance = start.distance_to(&current_point);
            let threshold = self.settings.min_distance as f32;

            if distance > threshold {
                // Start tracking
                self.state = TrackerState::Tracking;
                let context = GestureContext::new(start);
                self.emit_event(TrackerEvent::GestureStarted(context));
                debug!("Gesture started after {}px movement", distance);
            } else {
                return; // Don't process yet
            }
        }

        // Process movement
        let last_effective = self.last_effective_point.unwrap();
        let effective_distance = last_effective.distance_to(&current_point);
        let effective_threshold = self.settings.effective_move as f32;

        if effective_distance > effective_threshold {
            // Calculate direction
            let vector = last_effective.vector_to(&current_point);
            let use_8dir = self.settings.enable_8_direction
                && self.current_gesture.as_ref().map_or(false, |g| g.len() == 0);

            let dir = if use_8dir {
                // Use 8-direction for first stroke
                crate::core::parser::calculate_8direction(&vector)
            } else {
                crate::core::parser::calculate_4direction(&vector)
            };

            // Add direction to gesture
            if self.current_gesture.is_some() {
                let old_len = self.current_gesture.as_ref().unwrap().len();
                self.current_gesture.as_mut().unwrap().add_direction(dir);
                let new_len = self.current_gesture.as_ref().unwrap().len();

                if new_len != old_len {
                    debug!("Gesture direction added: {:?}", dir);
                    self.emit_event(TrackerEvent::GestureChanged(self.current_gesture.as_ref().unwrap().clone()));
                }
            }

            self.last_effective_point = Some(current_point);
        }

        self.last_point = Some(current_point);
        self.last_move_time = Some(Instant::now());
    }

    /// Handle mouse button up
    fn on_mouse_up(&mut self) {
        debug!("Mouse up");

        if let Some(gesture) = self.current_gesture.take() {
            if gesture.len() > 0 {
                debug!("Gesture completed: {} directions", gesture.len());
                self.emit_event(TrackerEvent::GestureCompleted(gesture));
            } else {
                debug!("Gesture cancelled (no directions)");
                self.emit_event(TrackerEvent::GestureCancelled);
            }
        }

        self.reset();
    }

    /// Handle gesture modifier
    fn on_modifier(&mut self, modifier: GestureModifier) {
        if self.state == TrackerState::Tracking {
            debug!("Modifier detected: {:?}", modifier);
            if let Some(ref mut gesture) = self.current_gesture {
                gesture.add_modifier(modifier);
            }
            self.emit_event(TrackerEvent::ModifierDetected(modifier));
        }
    }

    /// Check for timeout
    pub fn check_timeout(&mut self) -> bool {
        if self.state == TrackerState::Tracking {
            if let Some(last_time) = self.last_move_time {
                let elapsed = last_time.elapsed();
                let timeout = Duration::from_millis(self.settings.stay_timeout as u64);

                if elapsed > timeout {
                    debug!("Gesture timeout after {:?}", elapsed);
                    self.emit_event(TrackerEvent::GestureCancelled);
                    self.reset();
                    return true;
                }
            }
        }
        false
    }

    /// Reset the tracker
    fn reset(&mut self) {
        self.state = TrackerState::Idle;
        self.start_point = None;
        self.last_point = None;
        self.last_effective_point = None;
        self.current_gesture = None;
        self.mouse_down_time = None;
        self.last_move_time = None;
    }

    /// Emit an event
    fn emit_event(&self, event: TrackerEvent) {
        if let Some(ref callback) = self.event_callback {
            callback(event);
        }
    }

    /// Get the current state
    pub fn state(&self) -> &TrackerState {
        &self.state
    }

    /// Get the current gesture
    pub fn current_gesture(&self) -> Option<&Gesture> {
        self.current_gesture.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let settings = Settings::default();
        let tracker = PathTracker::new(settings);
        assert_eq!(tracker.state(), &TrackerState::Idle);
    }

    #[test]
    fn test_tracker_initial_move() {
        let settings = Settings {
            min_distance: 5,
            ..Default::default()
        };
        let mut tracker = PathTracker::new(settings);

        // Simulate mouse down
        tracker.on_mouse_down(100, 100, GestureTriggerButton::Middle);

        // Small movement (should not start tracking)
        tracker.on_mouse_move(102, 102);
        assert_eq!(tracker.state(), &TrackerState::Capturing);

        // Large movement (should start tracking)
        tracker.on_mouse_move(110, 110);
        assert_eq!(tracker.state(), &TrackerState::Tracking);
    }
}
