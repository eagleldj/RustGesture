//! Input simulation module
//!
//! This module provides simulation for keyboard, mouse, and window commands.

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use tracing::{debug, info};
use anyhow::anyhow;

use crate::config::config::{KeyboardAction, MouseAction, WindowAction, WindowCommand, MouseButton, MouseActionType, RunAction};

/// Tag for simulated input events (to prevent feedback loops)
pub const SIMULATED_EVENT_TAG: usize = 19900620;

/// Input simulator
#[derive(Clone, Copy)]
pub struct InputSimulator;

impl InputSimulator {
    /// Create a new input simulator
    pub fn new() -> Self {
        Self
    }

    /// Simulate keyboard input
    pub fn simulate_keyboard(&self, action: &KeyboardAction) -> anyhow::Result<()> {
        debug!("Simulating keyboard: {:?}", action.keys);

        if action.keys.is_empty() {
            return Ok(());
        }

        // Step 1: Press all keys down in order
        for key in &action.keys {
            self.send_key(key, true)?;
        }

        // Small delay to ensure the keys are registered
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Step 2: Release all keys in reverse order
        for key in action.keys.iter().rev() {
            self.send_key(key, false)?;
        }

        Ok(())
    }

    /// Send a key event
    fn send_key(&self, key: &str, down: bool) -> anyhow::Result<()> {
        let vk = self.map_key_to_vk(key)?;
        debug!("Sending key: {:?} ({})", key, if down { "down" } else { "up" });

        unsafe {
            let mut inputs = [KEYBDINPUT::default()];
            inputs[0].wVk = vk;
            inputs[0].dwFlags = if down {
                KEYBD_EVENT_FLAGS(0)
            } else {
                KEYBD_EVENT_FLAGS(0x0002) // KEYEVENTF_KEYUP
            };
            inputs[0].dwExtraInfo = SIMULATED_EVENT_TAG as usize;

            let input = INPUT {
                r#type: INPUT_TYPE(1), // INPUT_KEYBOARD
                Anonymous: INPUT_0 {
                    ki: inputs[0],
                },
            };

            let n = SendInput(
                &[input],
                std::mem::size_of::<INPUT>() as i32,
            );

            if n == 0 {
                return Err(anyhow!("SendInput failed: {:?}", GetLastError()));
            }
        }

        Ok(())
    }

    /// Map key string to virtual key code
    fn map_key_to_vk(&self, key: &str) -> anyhow::Result<VIRTUAL_KEY> {
        match key.to_uppercase().as_str() {
            "VK_CONTROL" | "CONTROL" | "CTRL" => Ok(VIRTUAL_KEY(VK_CONTROL.0)),
            "VK_LCONTROL" | "LCONTROL" | "LCTRL" => Ok(VIRTUAL_KEY(VK_LCONTROL.0)),
            "VK_RCONTROL" | "RCONTROL" | "RCTRL" => Ok(VIRTUAL_KEY(VK_RCONTROL.0)),
            "VK_SHIFT" | "SHIFT" => Ok(VIRTUAL_KEY(VK_SHIFT.0)),
            "VK_LSHIFT" | "LSHIFT" => Ok(VIRTUAL_KEY(VK_LSHIFT.0)),
            "VK_RSHIFT" | "RSHIFT" => Ok(VIRTUAL_KEY(VK_RSHIFT.0)),
            "VK_ALT" | "ALT" => Ok(VIRTUAL_KEY(VK_MENU.0)),
            "VK_LMENU" | "LALT" => Ok(VIRTUAL_KEY(VK_LMENU.0)),
            "VK_RMENU" | "RALT" => Ok(VIRTUAL_KEY(VK_RMENU.0)),
            "VK_LWIN" | "LWIN" => Ok(VIRTUAL_KEY(VK_LWIN.0)),
            "VK_RWIN" | "RWIN" => Ok(VIRTUAL_KEY(VK_RWIN.0)),
            "VK_BACK" | "BACKSPACE" => Ok(VIRTUAL_KEY(0x08)),
            "VK_TAB" | "TAB" => Ok(VIRTUAL_KEY(0x09)),
            "VK_RETURN" | "ENTER" => Ok(VIRTUAL_KEY(0x0D)),
            "VK_ESCAPE" | "ESC" => Ok(VIRTUAL_KEY(0x1B)),
            "VK_SPACE" | "SPACE" => Ok(VIRTUAL_KEY(0x20)),
            // Letter keys (A-Z): VK_0x41 to VK_0x5A
            "VK_A" | "A" => Ok(VIRTUAL_KEY(0x41)),
            "VK_B" | "B" => Ok(VIRTUAL_KEY(0x42)),
            "VK_C" | "C" => Ok(VIRTUAL_KEY(0x43)),
            "VK_D" | "D" => Ok(VIRTUAL_KEY(0x44)),
            "VK_E" | "E" => Ok(VIRTUAL_KEY(0x45)),
            "VK_F" | "F" => Ok(VIRTUAL_KEY(0x46)),
            "VK_G" | "G" => Ok(VIRTUAL_KEY(0x47)),
            "VK_H" | "H" => Ok(VIRTUAL_KEY(0x48)),
            "VK_I" | "I" => Ok(VIRTUAL_KEY(0x49)),
            "VK_J" | "J" => Ok(VIRTUAL_KEY(0x4A)),
            "VK_K" | "K" => Ok(VIRTUAL_KEY(0x4B)),
            "VK_L" | "L" => Ok(VIRTUAL_KEY(0x4C)),
            "VK_M" | "M" => Ok(VIRTUAL_KEY(0x4D)),
            "VK_N" | "N" => Ok(VIRTUAL_KEY(0x4E)),
            "VK_O" | "O" => Ok(VIRTUAL_KEY(0x4F)),
            "VK_P" | "P" => Ok(VIRTUAL_KEY(0x50)),
            "VK_Q" | "Q" => Ok(VIRTUAL_KEY(0x51)),
            "VK_R" | "R" => Ok(VIRTUAL_KEY(0x52)),
            "VK_S" | "S" => Ok(VIRTUAL_KEY(0x53)),
            "VK_T" | "T" => Ok(VIRTUAL_KEY(0x54)),
            "VK_U" | "U" => Ok(VIRTUAL_KEY(0x55)),
            "VK_V" | "V" => Ok(VIRTUAL_KEY(0x56)),
            "VK_W" | "W" => Ok(VIRTUAL_KEY(0x57)),
            "VK_X" | "X" => Ok(VIRTUAL_KEY(0x58)),
            "VK_Y" | "Y" => Ok(VIRTUAL_KEY(0x59)),
            "VK_Z" | "Z" => Ok(VIRTUAL_KEY(0x5A)),
            // Number keys (0-9): VK_0x30 to VK_0x39
            "VK_0" | "0" => Ok(VIRTUAL_KEY(0x30)),
            "VK_1" | "1" => Ok(VIRTUAL_KEY(0x31)),
            "VK_2" | "2" => Ok(VIRTUAL_KEY(0x32)),
            "VK_3" | "3" => Ok(VIRTUAL_KEY(0x33)),
            "VK_4" | "4" => Ok(VIRTUAL_KEY(0x34)),
            "VK_5" | "5" => Ok(VIRTUAL_KEY(0x35)),
            "VK_6" | "6" => Ok(VIRTUAL_KEY(0x36)),
            "VK_7" | "7" => Ok(VIRTUAL_KEY(0x37)),
            "VK_8" | "8" => Ok(VIRTUAL_KEY(0x38)),
            "VK_9" | "9" => Ok(VIRTUAL_KEY(0x39)),
            // Function keys
            "VK_F1" | "F1" => Ok(VIRTUAL_KEY(0x70)),
            "VK_F2" | "F2" => Ok(VIRTUAL_KEY(0x71)),
            "VK_F3" | "F3" => Ok(VIRTUAL_KEY(0x72)),
            "VK_F4" | "F4" => Ok(VIRTUAL_KEY(0x73)),
            "VK_F5" | "F5" => Ok(VIRTUAL_KEY(0x74)),
            "VK_F6" | "F6" => Ok(VIRTUAL_KEY(0x75)),
            "VK_F7" | "F7" => Ok(VIRTUAL_KEY(0x76)),
            "VK_F8" | "F8" => Ok(VIRTUAL_KEY(0x77)),
            "VK_F9" | "F9" => Ok(VIRTUAL_KEY(0x78)),
            "VK_F10" | "F10" => Ok(VIRTUAL_KEY(0x79)),
            "VK_F11" | "F11" => Ok(VIRTUAL_KEY(0x7A)),
            "VK_F12" | "F12" => Ok(VIRTUAL_KEY(0x7B)),
            // Arrow keys
            "VK_LEFT" | "LEFT" => Ok(VIRTUAL_KEY(0x25)),
            "VK_UP" | "UP" => Ok(VIRTUAL_KEY(0x26)),
            "VK_RIGHT" | "RIGHT" => Ok(VIRTUAL_KEY(0x27)),
            "VK_DOWN" | "DOWN" => Ok(VIRTUAL_KEY(0x28)),
            // Navigation keys
            "VK_HOME" | "HOME" => Ok(VIRTUAL_KEY(0x24)),
            "VK_END" | "END" => Ok(VIRTUAL_KEY(0x23)),
            "VK_PRIOR" | "PAGEUP" => Ok(VIRTUAL_KEY(0x21)),
            "VK_NEXT" | "PAGEDOWN" => Ok(VIRTUAL_KEY(0x22)),
            "VK_INSERT" | "INSERT" => Ok(VIRTUAL_KEY(0x2D)),
            "VK_DELETE" | "DELETE" => Ok(VIRTUAL_KEY(0x2E)),
            // Add more key mappings as needed
            _ => {
                // Try to parse as virtual key code
                if let Ok(code) = key.parse::<u16>() {
                    Ok(VIRTUAL_KEY(code))
                } else {
                    Err(anyhow!("Unknown key: {}", key))
                }
            }
        }
    }

    /// Simulate mouse input
    pub fn simulate_mouse(&self, action: &MouseAction) -> anyhow::Result<()> {
        debug!("Simulating mouse: {:?} {:?}", action.button, action.action_type);

        match action.action_type {
            MouseActionType::Click => {
                self.send_mouse_click(&action.button)?;
            }
            MouseActionType::DoubleClick => {
                self.send_mouse_click(&action.button)?;
                std::thread::sleep(std::time::Duration::from_millis(50)); // Delay between clicks
                self.send_mouse_click(&action.button)?;
            }
        }

        Ok(())
    }

    /// Send a mouse click
    fn send_mouse_click(&self, button: &MouseButton) -> anyhow::Result<()> {
        let (flags_down, flags_up) = match button {
            MouseButton::Left => (MOUSE_EVENT_FLAGS(0x0002), MOUSE_EVENT_FLAGS(0x0004)), // MOUSEEVENTF_LEFTDOWN/UP
            MouseButton::Right => (MOUSE_EVENT_FLAGS(0x0008), MOUSE_EVENT_FLAGS(0x0010)), // MOUSEEVENTF_RIGHTDOWN/UP
            MouseButton::Middle => (MOUSE_EVENT_FLAGS(0x0020), MOUSE_EVENT_FLAGS(0x0040)), // MOUSEEVENTF_MIDDLEDOWN/UP
            MouseButton::X1 => (MOUSE_EVENT_FLAGS(0x0080), MOUSE_EVENT_FLAGS(0x0100)), // MOUSEEVENTF_XDOWN/UP
            MouseButton::X2 => (MOUSE_EVENT_FLAGS(0x0080), MOUSE_EVENT_FLAGS(0x0100)), // MOUSEEVENTF_XDOWN/UP (with different data)
        };

        unsafe {
            let mut inputs = [
                MOUSEINPUT::default(),
                MOUSEINPUT::default(),
            ];

            // Mouse down
            inputs[0].dwFlags = flags_down;
            inputs[0].dwExtraInfo = SIMULATED_EVENT_TAG as usize;

            // Set mouse data for X1/X2 buttons
            if matches!(button, MouseButton::X1 | MouseButton::X2) {
                let data = if matches!(button, MouseButton::X1) { 1u32 } else { 2u32 };
                inputs[0].mouseData = data;
                inputs[1].mouseData = data;
            }

            // Mouse up
            inputs[1].dwFlags = flags_up;
            inputs[1].dwExtraInfo = SIMULATED_EVENT_TAG as usize;

            let input_0 = INPUT {
                r#type: INPUT_TYPE(0), // INPUT_MOUSE
                Anonymous: INPUT_0 {
                    mi: inputs[0],
                },
            };

            let input_1 = INPUT {
                r#type: INPUT_TYPE(0),
                Anonymous: INPUT_0 {
                    mi: inputs[1],
                },
            };

            let n = SendInput(
                &[input_0, input_1],
                std::mem::size_of::<INPUT>() as i32,
            );

            if n == 0 {
                return Err(anyhow!("SendInput (mouse) failed: {:?}", GetLastError()));
            }
        }

        Ok(())
    }

    /// Simulate window command
    pub fn simulate_window_command(&self, action: &WindowAction) -> anyhow::Result<()> {
        debug!("Simulating window command: {:?}", action.command);

        unsafe {
            let hwnd = GetForegroundWindow();

            match action.command {
                WindowCommand::Minimize => {
                    let _ = ShowWindow(hwnd, SW_SHOWMINIMIZED);
                    debug!("Window minimized");
                }
                WindowCommand::Maximize => {
                    let _ = ShowWindow(hwnd, SW_SHOWMAXIMIZED);
                    debug!("Window maximized");
                }
                WindowCommand::Restore => {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    debug!("Window restored");
                }
                WindowCommand::Close => {
                    SendMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
                    debug!("Window close message sent");
                }
                WindowCommand::ShowDesktop => {
                    // Simulate Win+D using SendInput
                    self.send_key_vk(VK_LWIN, true)?;
                    self.send_key_vk(VK_D, true)?;
                    self.send_key_vk(VK_D, false)?;
                    self.send_key_vk(VK_LWIN, false)?;
                    debug!("Show desktop simulated");
                }
            }
        }

        Ok(())
    }

    /// Send a key event using virtual key code directly
    fn send_key_vk(&self, vk: VIRTUAL_KEY, down: bool) -> anyhow::Result<()> {
        unsafe {
            let mut kbd_input = KEYBDINPUT::default();
            kbd_input.wVk = vk;
            kbd_input.dwFlags = if down {
                KEYBD_EVENT_FLAGS(0)
            } else {
                KEYBD_EVENT_FLAGS(0x0002) // KEYEVENTF_KEYUP
            };
            kbd_input.dwExtraInfo = SIMULATED_EVENT_TAG as usize;

            let input = INPUT {
                r#type: INPUT_TYPE(1), // INPUT_KEYBOARD
                Anonymous: INPUT_0 {
                    ki: kbd_input,
                },
            };

            let n = SendInput(
                &[input],
                std::mem::size_of::<INPUT>() as i32,
            );

            if n == 0 {
                return Err(anyhow!("SendInput (VK) failed: {:?}", GetLastError()));
            }
        }

        Ok(())
    }

    /// Run a program
    pub fn run_program(&self, action: &RunAction) -> anyhow::Result<()> {
        info!("Running program: {} {:?}", action.command, action.args);

        use std::process::Command;
        let mut cmd = Command::new(&action.command);
        if let Some(args) = &action.args {
            // args is a single string, split by spaces
            for arg in args.split_whitespace() {
                cmd.arg(arg);
            }
        }

        match cmd.spawn() {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Failed to run program: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::config::{KeyboardAction, MouseAction, WindowAction, WindowCommand, MouseButton, MouseActionType, RunAction};

    #[test]
    fn test_input_simulator_creation() {
        let simulator = InputSimulator::new();
        assert!(true);
    }

    #[test]
    fn test_map_key_to_vk() {
        let simulator = InputSimulator::new();
        assert!(simulator.map_key_to_vk("CONTROL").is_ok());
        assert!(simulator.map_key_to_vk("SHIFT").is_ok());
        assert!(simulator.map_key_to_vk("UNKNOWN_KEY").is_err());
    }
}

