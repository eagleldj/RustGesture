//! Hook callback implementation
//!
//! This module connects Windows mouse hook events to the gesture recognition system.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::recognizer::SharedRecognizer;
use crate::winapi::hook::{set_processing_mouse_moves, MouseEvent, MouseHookCallback};
use std::sync::mpsc;

/// Hook callback that connects mouse events to gesture recognition
pub struct GestureHookCallback {
    recognizer: SharedRecognizer,
    enabled: Arc<AtomicBool>,
    // Use a channel to send events asynchronously
    _event_sender: mpsc::Sender<MouseEvent>,
}

impl GestureHookCallback {
    /// Create a new gesture hook callback
    pub fn new(recognizer: SharedRecognizer, enabled: Arc<AtomicBool>) -> Self {
        info!("GestureHookCallback created with multi-button support (Right/Middle/X1/X2)");

        // Create a channel for async event processing
        let (event_sender, event_receiver) = mpsc::channel::<MouseEvent>();
        let recognizer_clone = recognizer.clone();

        // Spawn a thread to process events asynchronously
        std::thread::spawn(move || {
            info!("Event processing thread started");
            for event in event_receiver {
                if let Ok(mut recognizer) = recognizer_clone.try_lock() {
                    recognizer.handle_mouse_event(&event);
                }
            }
            info!("Event processing thread ended");
        });

        Self {
            recognizer,
            enabled,
            _event_sender: event_sender,
        }
    }
}

impl MouseHookCallback for GestureHookCallback {
    /// Called when a mouse event occurs
    /// CRITICAL: This MUST be ultra-fast - just send to channel and return!
    fn on_mouse_event(&self, event: &MouseEvent) -> bool {
        // Fast atomic check
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        // Log button events and set processing state
        match event {
            MouseEvent::RightButtonDown(x, y) => {
                info!("🖱️  Right button DOWN at ({}, {})", x, y);
                set_processing_mouse_moves(true);
            }
            MouseEvent::MiddleButtonDown(x, y) => {
                info!("🖱️  Middle button DOWN at ({}, {})", x, y);
                set_processing_mouse_moves(true);
            }
            MouseEvent::XButtonDown(x, y, btn) => {
                info!("🖱️  X{} button DOWN at ({}, {})", btn, x, y);
                set_processing_mouse_moves(true);
            }
            MouseEvent::RightButtonUp(_, _) => {
                info!("🖱️  Right button UP");
                set_processing_mouse_moves(false);
            }
            MouseEvent::MiddleButtonUp(_, _) => {
                info!("🖱️  Middle button UP");
                set_processing_mouse_moves(false);
            }
            MouseEvent::XButtonUp(_, _, btn) => {
                info!("🖱️  X{} button UP", btn);
                set_processing_mouse_moves(false);
            }
            MouseEvent::MouseMove(x, y) => {
                // Only log mouse moves when tracking (trigger button pressed)
                if crate::winapi::hook::is_processing_mouse_moves() {
                    debug!("📍 Mouse move to ({}, {})", x, y);
                }
            }
            _ => {}
        }

        // Just send to channel and return immediately - don't wait for processing!
        let _ = self._event_sender.send(*event);

        false
    }
}
