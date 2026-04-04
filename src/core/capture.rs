//! Gesture capture mode for settings UI
//!
//! Provides global state for capturing gestures in the settings dialog.
//! When capture mode is active, the recognizer stores the captured direction
//! sequence and trigger button instead of executing the matched action.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::core::gesture::GestureTriggerButton;

/// Captured gesture result including directions and trigger button
#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub directions: Vec<String>,
    pub trigger_button: GestureTriggerButton,
}

static CAPTURE_MODE: AtomicBool = AtomicBool::new(false);
static CAPTURE_RESULT: Mutex<Option<CaptureResult>> = Mutex::new(None);

/// Enable capture mode. The next completed gesture will be captured.
pub fn start_capture() {
    // Clear any previous result
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        *result = None;
    }
    CAPTURE_MODE.store(true, Ordering::SeqCst);
}

/// Cancel capture mode without waiting for a gesture.
pub fn cancel_capture() {
    CAPTURE_MODE.store(false, Ordering::SeqCst);
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        *result = None;
    }
}

/// Check if currently in capture mode.
pub fn is_capture_mode() -> bool {
    CAPTURE_MODE.load(Ordering::SeqCst)
}

/// Store a captured gesture result (directions + trigger button).
/// Called by the recognizer callback when a gesture is completed in capture mode.
pub fn set_capture_result(directions: Vec<String>, trigger_button: GestureTriggerButton) {
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        *result = Some(CaptureResult {
            directions,
            trigger_button,
        });
    }
    CAPTURE_MODE.store(false, Ordering::SeqCst);
}

/// Take the captured gesture result (returns Some if a gesture was captured, None otherwise).
/// This consumes the result.
pub fn take_capture_result() -> Option<CaptureResult> {
    if let Ok(mut result) = CAPTURE_RESULT.lock() {
        result.take()
    } else {
        None
    }
}
