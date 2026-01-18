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

        // TODO: Map key names to virtual key codes
        // For now, this is a placeholder
        for key in &action.keys {
            self.send_key(key, true)?;
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
                    ShowWindow(hwnd, SW_SHOWMINIMIZED);
                    debug!("Window minimized");
                }
                WindowCommand::Maximize => {
                    ShowWindow(hwnd, SW_SHOWMAXIMIZED);
                    debug!("Window maximized");
                }
                WindowCommand::Restore => {
                    ShowWindow(hwnd, SW_RESTORE);
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

