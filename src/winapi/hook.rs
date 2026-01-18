//! Windows hook module
//!
//! This module handles low-level mouse and keyboard hooks using Windows API.

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use std::sync::Arc;
use std::sync::Mutex;

use crate::core::gesture::{GestureTriggerButton, Point};
use anyhow::{Result, anyhow};

// Global callback for mouse hook
// Using a Mutex to allow thread-safe access
static MOUSE_HOOK_CALLBACK: Mutex<Option<Box<dyn MouseHookCallback>>> = Mutex::new(None);

/// Mouse event types
#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    MouseMove(i32, i32),
    LeftButtonDown(i32, i32),
    LeftButtonUp(i32, i32),
    RightButtonDown(i32, i32),
    RightButtonUp(i32, i32),
    MiddleButtonDown(i32, i32),
    MiddleButtonUp(i32, i32),
    XButtonDown(i32, i32, u16), // x, y, button_number
    XButtonUp(i32, i32, u16),
    MouseWheel(i32, i32, i32), // x, y, delta
    MouseHWheel(i32, i32, i32),
}

impl MouseEvent {
    /// Get the position of the mouse event
    pub fn position(&self) -> (i32, i32) {
        match self {
            MouseEvent::MouseMove(x, y) => (*x, *y),
            MouseEvent::LeftButtonDown(x, y) => (*x, *y),
            MouseEvent::LeftButtonUp(x, y) => (*x, *y),
            MouseEvent::RightButtonDown(x, y) => (*x, *y),
            MouseEvent::RightButtonUp(x, y) => (*x, *y),
            MouseEvent::MiddleButtonDown(x, y) => (*x, *y),
            MouseEvent::MiddleButtonUp(x, y) => (*x, *y),
            MouseEvent::XButtonDown(x, y, _) => (*x, *y),
            MouseEvent::XButtonUp(x, y, _) => (*x, *y),
            MouseEvent::MouseWheel(x, y, _) => (*x, *y),
            MouseEvent::MouseHWheel(x, y, _) => (*x, *y),
        }
    }
}

/// Mouse hook callback trait
pub trait MouseHookCallback: Send + Sync {
    /// Called when a mouse event occurs
    fn on_mouse_event(&self, event: &MouseEvent) -> bool;
}

/// Dummy callback for placeholder
struct DummyCallback;

impl MouseHookCallback for DummyCallback {
    fn on_mouse_event(&self, _event: &MouseEvent) -> bool {
        false
    }
}

/// Low-level mouse hook
pub struct MouseHook {
    hook: Option<HHOOK>,
    callback: Option<Box<dyn MouseHookCallback>>,
}

impl MouseHook {
    /// Create a new mouse hook
    pub fn new() -> Self {
        Self {
            hook: None,
            callback: None,
        }
    }

    /// Set the callback for mouse events
    pub fn set_callback(&mut self, callback: Box<dyn MouseHookCallback>) {
        // Store callback in global static variable
        let mut global_callback = MOUSE_HOOK_CALLBACK.lock().unwrap();
        *global_callback = Some(callback);
        self.callback = Some(Box::new(DummyCallback));
    }

    /// Install the mouse hook
    pub fn install(&mut self) -> Result<()> {
        if self.hook.is_some() {
            return Err(anyhow!("Hook already installed"));
        }

        unsafe {
            let hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(Self::hook_proc),
                HINSTANCE::default(),
                0,
            );

            match hook {
                Ok(h) => {
                    if h.is_invalid() {
                        return Err(anyhow!("Failed to set mouse hook: invalid handle"));
                    }
                    self.hook = Some(h);
                }
                Err(e) => {
                    return Err(anyhow!("Failed to set mouse hook: {:?}", e));
                }
            }
        }

        Ok(())
    }

    /// Uninstall the mouse hook
    pub fn uninstall(&mut self) -> Result<()> {
        if let Some(hook) = self.hook.take() {
            unsafe {
                if UnhookWindowsHookEx(hook).is_ok() {
                    // Clear the global callback
                    let mut global_callback = MOUSE_HOOK_CALLBACK.lock().unwrap();
                    *global_callback = None;
                    Ok(())
                } else {
                    Err(anyhow!("Failed to unhook mouse hook"))
                }
            }
        } else {
            Err(anyhow!("No hook installed"))
        }
    }

    /// Hook procedure (called by Windows)
    /// IMPORTANT: This must return VERY quickly to avoid mouse lag!
    unsafe extern "system" fn hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        // Always call next hook first to ensure minimal latency
        let result = CallNextHookEx(HHOOK::default(), n_code, w_param, l_param);

        if n_code as u32 == HC_ACTION {
            // Extract hook structure
            let hook_struct = *(l_param.0 as *const MSLLHOOKSTRUCT);

            // Check if this is a simulated event (to avoid feedback loops)
            const SIMULATED_EVENT_TAG: usize = 19900620;
            let extra_info = hook_struct.dwExtraInfo as usize;
            if extra_info == SIMULATED_EVENT_TAG {
                return result;
            }

            // Convert to our MouseEvent type
            let event = Self::convert_mouse_event(w_param.0 as u32, &hook_struct);

            // Dispatch to callback (if any) - use try_lock for speed
            if let Ok(global_callback) = MOUSE_HOOK_CALLBACK.try_lock() {
                if let Some(ref callback) = *global_callback {
                    // Send event to async channel - this is ultra fast!
                    let _ = callback.on_mouse_event(&event);
                }
            }
        }

        result
    }

    /// Convert Windows mouse message to MouseEvent
    unsafe fn convert_mouse_event(msg: u32, hook_struct: &MSLLHOOKSTRUCT) -> MouseEvent {
        let x = hook_struct.pt.x;
        let y = hook_struct.pt.y;
        let mouse_data = hook_struct.mouseData;

        match msg {
            WM_MOUSEMOVE => MouseEvent::MouseMove(x, y),
            WM_LBUTTONDOWN => MouseEvent::LeftButtonDown(x, y),
            WM_LBUTTONUP => MouseEvent::LeftButtonUp(x, y),
            WM_RBUTTONDOWN => MouseEvent::RightButtonDown(x, y),
            WM_RBUTTONUP => MouseEvent::RightButtonUp(x, y),
            WM_MBUTTONDOWN => MouseEvent::MiddleButtonDown(x, y),
            WM_MBUTTONUP => MouseEvent::MiddleButtonUp(x, y),
            WM_XBUTTONDOWN => {
                let button = (mouse_data >> 16) as u16;
                MouseEvent::XButtonDown(x, y, button)
            }
            WM_XBUTTONUP => {
                let button = (mouse_data >> 16) as u16;
                MouseEvent::XButtonUp(x, y, button)
            }
            WM_MOUSEWHEEL => {
                let delta = ((mouse_data >> 16) as i16) as i32;
                MouseEvent::MouseWheel(x, y, delta)
            }
            WM_MOUSEHWHEEL => {
                let delta = ((mouse_data >> 16) as i16) as i32;
                MouseEvent::MouseHWheel(x, y, delta)
            }
            _ => MouseEvent::MouseMove(x, y),
        }
    }
}

impl Default for MouseHook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_event_position() {
        let event = MouseEvent::MouseMove(100, 200);
        assert_eq!(event.position(), (100, 200));
    }
}
