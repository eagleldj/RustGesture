//! Hook callback implementation
//!
//! This module connects Windows mouse hook events to the gesture recognition system.

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, info, warn};
use anyhow::anyhow;

use crate::core::gesture::GestureTriggerButton;
use crate::core::recognizer::SharedRecognizer;
use crate::winapi::hook::{MouseEvent, MouseHookCallback, set_processing_mouse_moves};
use crate::config::config::TriggerButton;
use std::sync::mpsc;

/// Hook callback that connects mouse events to gesture recognition
pub struct GestureHookCallback {
    recognizer: SharedRecognizer,
    enabled: Arc<AtomicBool>,
    trigger_button: TriggerButton,
    // Use a channel to send events asynchronously
    _event_sender: mpsc::Sender<MouseEvent>,
}

impl GestureHookCallback {
    /// Create a new gesture hook callback
    pub fn new(
        recognizer: SharedRecognizer,
        trigger_button: TriggerButton,
        enabled: Arc<AtomicBool>
    ) -> Self {
        info!("GestureHookCallback created with async channel, trigger button: {:?}", trigger_button);

        // Create a channel for async event processing
        let (event_sender, event_receiver) = mpsc::channel::<MouseEvent>();
        let recognizer_clone = recognizer.clone();

        // Spawn a thread to process events asynchronously
        std::thread::spawn(move || {
            info!("Event processing thread started");
            for event in event_receiver {
                if let Ok(mut recognizer) = recognizer_clone.lock() {
                    recognizer.handle_mouse_event(&event);
                }
            }
            info!("Event processing thread ended");
        });

        Self {
            recognizer,
            enabled,
            trigger_button,
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

        // Log button events
        match event {
            MouseEvent::RightButtonDown(x, y) => {
                if self.trigger_button == TriggerButton::Right {
                    info!("🖱️  Right button DOWN at ({}, {})", x, y);
                    set_processing_mouse_moves(true);
                }
            }
            MouseEvent::MiddleButtonDown(x, y) => {
                if self.trigger_button == TriggerButton::Middle {
                    info!("🖱️  Middle button DOWN at ({}, {})", x, y);
                    set_processing_mouse_moves(true);
                }
            }
            MouseEvent::XButtonDown(x, y, btn) => {
                if (self.trigger_button == TriggerButton::X1 || self.trigger_button == TriggerButton::X2) {
                    info!("🖱️  X{} button DOWN at ({}, {})", btn, x, y);
                    set_processing_mouse_moves(true);
                }
            }
            MouseEvent::RightButtonUp(_, _) => {
                if self.trigger_button == TriggerButton::Right {
                    info!("🖱️  Right button UP");
                    set_processing_mouse_moves(false);
                }
            }
            MouseEvent::MiddleButtonUp(_, _) => {
                if self.trigger_button == TriggerButton::Middle {
                    info!("�️  Middle button UP");
                    set_processing_mouse_moves(false);
                }
            }
            MouseEvent::XButtonUp(_, _, _) => {
                if self.trigger_button == TriggerButton::X1 || self.trigger_button == TriggerButton::X2 {
                    info!("🖱️  X button UP");
                    set_processing_mouse_moves(false);
                }
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
