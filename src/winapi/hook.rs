//! Windows hook module
//!
//! This module handles low-level mouse and keyboard hooks using Windows API.

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::PCWSTR;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::info;

use crate::core::gesture::{GestureTriggerButton, Point};
use anyhow::{Result, anyhow};

// Global callback for mouse hook
// Using a Mutex to allow thread-safe access
static MOUSE_HOOK_CALLBACK: Mutex<Option<Box<dyn MouseHookCallback>>> = Mutex::new(None);

// Performance monitoring: count how many times hook is called
static HOOK_CALL_COUNT: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

// Performance monitoring: last hook duration in nanoseconds
static LAST_HOOK_DURATION_NS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

// Get performance stats
pub fn get_hook_stats() -> (u64, u64) {
    let count = HOOK_CALL_COUNT.load(std::sync::atomic::Ordering::Relaxed);
    let duration_ns = LAST_HOOK_DURATION_NS.load(std::sync::atomic::Ordering::Relaxed);
    (count, duration_ns)
}

// Global flag to track if we should process mouse moves
// Only process moves when trigger button is pressed
static PROCESSING_MOUSE_MOVES: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

// Counter for sampling mouse moves to reduce frequency
static MOUSE_MOVE_COUNTER: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

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

/// Set whether to process mouse move events
/// Should be called when trigger button is pressed/released
pub fn set_processing_mouse_moves(process: bool) {
    PROCESSING_MOUSE_MOVES.store(process, std::sync::atomic::Ordering::Relaxed);
}

/// Check if currently processing mouse moves (for logging)
pub fn is_processing_mouse_moves() -> bool {
    PROCESSING_MOUSE_MOVES.load(std::sync::atomic::Ordering::Relaxed)
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
            // Get the actual HINSTANCE for this executable (not the DLL default)
            let hinstance = GetModuleHandleW(PCWSTR::null())
                .map_err(|e| anyhow!("Failed to get module handle: {:?}", e))?;

            info!("Installing mouse hook with HINSTANCE: {:?}", hinstance);

            let hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(Self::hook_proc),
                hinstance,  // Use real HINSTANCE, not default!
                0,
            );

            match hook {
                Ok(h) => {
                    if h.is_invalid() {
                        let error = GetLastError();
                        return Err(anyhow!("Failed to set mouse hook: invalid handle, error: {:?}", error));
                    }
                    info!("Mouse hook installed successfully: {:?}", h);
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
    /// With message loop running, hooks should work smoothly
    unsafe extern "system" fn hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        // Performance monitoring: start timing
        let start = std::time::Instant::now();
        HOOK_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Call next hook FIRST (critical for minimal latency)
        let result = CallNextHookEx(HHOOK::default(), n_code, w_param, l_param);

        if n_code as u32 == HC_ACTION {
            let msg = w_param.0 as u32;

            // Skip MouseMove when not tracking
            if msg == WM_MOUSEMOVE {
                if !PROCESSING_MOUSE_MOVES.load(std::sync::atomic::Ordering::Relaxed) {
                    // Record duration and return
                    let duration = start.elapsed().as_nanos() as u64;
                    LAST_HOOK_DURATION_NS.store(duration, std::sync::atomic::Ordering::Relaxed);
                    return result;
                }

                // Sample mouse moves (every 5th event)
                let count = MOUSE_MOVE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 5 != 0 {
                    let duration = start.elapsed().as_nanos() as u64;
                    LAST_HOOK_DURATION_NS.store(duration, std::sync::atomic::Ordering::Relaxed);
                    return result;
                }
            }

            // Check for simulated events
            const SIMULATED_EVENT_TAG: usize = 19900620;
            let hook_struct = *(l_param.0 as *const MSLLHOOKSTRUCT);
            let extra_info = hook_struct.dwExtraInfo as usize;
            if extra_info == SIMULATED_EVENT_TAG {
                let duration = start.elapsed().as_nanos() as u64;
                LAST_HOOK_DURATION_NS.store(duration, std::sync::atomic::Ordering::Relaxed);
                return result;
            }

            // Convert and dispatch event
            let event = Self::convert_mouse_event(msg, &hook_struct);

            if let Ok(global_callback) = MOUSE_HOOK_CALLBACK.try_lock() {
                if let Some(ref callback) = *global_callback {
                    let _ = callback.on_mouse_event(&event);
                }
            }
        }

        // Record duration
        let duration = start.elapsed().as_nanos() as u64;
        LAST_HOOK_DURATION_NS.store(duration, std::sync::atomic::Ordering::Relaxed);

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
