//! Windows API helper functions
//!
//! This module provides safe wrappers and helper functions for common Windows API operations.

#![allow(dead_code)]

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::gesture::Point;

/// Get the DPI scaling factor for the primary monitor
pub fn get_dpi() -> u32 {
    unsafe {
        let hdc = GetDC(HWND::default());
        if hdc.is_invalid() {
            return 96; // Default DPI
        }

        let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
        ReleaseDC(HWND::default(), hdc);

        dpi as u32
    }
}

/// Get DPI scaling factor as a multiplier (96 DPI = 1.0)
pub fn get_dpi_scaling() -> f64 {
    get_dpi() as f64 / 96.0
}

/// Get the foreground window handle
pub fn get_foreground_window() -> Option<HWND> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        None
    } else {
        Some(hwnd)
    }
}

/// Get the window rectangle
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            None
        }
    }
}

/// Get the desktop window handle
pub fn get_desktop_window() -> HWND {
    unsafe { GetDesktopWindow() }
}

/// Get the process ID for a window
pub fn get_window_process_id(hwnd: HWND) -> Option<u32> {
    unsafe {
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32));
        if process_id == 0 {
            None
        } else {
            Some(process_id)
        }
    }
}

/// Check if a window is in fullscreen mode
pub fn is_fullscreen(hwnd: HWND) -> bool {
    unsafe {
        let rect = match get_window_rect(hwnd) {
            Some(r) => r,
            None => return false,
        };

        let desktop = get_desktop_window();
        let desktop_rect = match get_window_rect(desktop) {
            Some(r) => r,
            None => return false,
        };

        // Check if window covers entire desktop
        if rect != desktop_rect {
            return false;
        }

        // Exclude special windows
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);

        if len > 0 {
            let class_name_str = String::from_utf16_lossy(&class_name[..len as usize]);
            match class_name_str.as_str() {
                "WorkerW" | "CanvasWindow" | "ImmersiveLauncher" | "Windows.UI.Core.CoreWindow" => {
                    return false;
                }
                _ => {}
            }
        }

        true
    }
}

/// Get the active window and its process ID
pub fn get_active_window_info() -> Option<(HWND, u32)> {
    let hwnd = get_foreground_window()?;
    let process_id = get_window_process_id(hwnd)?;
    Some((hwnd, process_id))
}

/// Get window from a point
pub fn window_from_point(point: &Point) -> Option<HWND> {
    unsafe {
        let hwnd = WindowFromPoint(POINT {
            x: point.x,
            y: point.y,
        });

        if hwnd.is_invalid() {
            None
        } else {
            Some(hwnd)
        }
    }
}

/// Get the executable name for a process ID
///
/// Note: This is a simplified version. In production, you'd use QueryFullProcessImageNameW
/// or EnumProcesses to get the actual executable name.
pub fn get_process_name(_process_id: u32) -> Option<String> {
    // TODO: Implement proper process name retrieval
    // For now, return None to avoid complexity
    None
}

/// Check if a mouse button is currently pressed
pub fn is_key_down(vkey: VIRTUAL_KEY) -> bool {
    unsafe {
        let state = GetAsyncKeyState(i32::from(vkey.0));
        (state as u16 & 0x8000) != 0
    }
}

/// Check if mouse buttons are swapped (left-handed mode)
pub fn is_mouse_button_swapped() -> bool {
    // TODO: Implement proper check using SystemParametersInfo
    // For now, always return false
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpi() {
        let dpi = get_dpi();
        assert!(dpi > 0);
    }

    #[test]
    fn test_dpi_scaling() {
        let scaling = get_dpi_scaling();
        assert!(scaling > 0.0);
    }
}
