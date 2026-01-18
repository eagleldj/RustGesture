//! Command executor module
//!
//! This module executes actions that are triggered by gestures.

use crate::config::config::Action;
use crate::winapi::input::InputSimulator;
use anyhow::Result;
use tracing::info;

/// Command executor
#[derive(Clone)]
pub struct CommandExecutor {
    simulator: InputSimulator,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Self {
        Self {
            simulator: InputSimulator::new(),
        }
    }

    /// Execute an action
    pub fn execute(&self, action: &Action) -> Result<()> {
        match action {
            Action::Keyboard(keyboard_action) => {
                info!("Executing keyboard action: {:?}", keyboard_action.keys);
                self.simulator.simulate_keyboard(keyboard_action)
            }
            Action::Mouse(mouse_action) => {
                info!("Executing mouse action: {:?} {:?}", mouse_action.button, mouse_action.action_type);
                self.simulator.simulate_mouse(mouse_action)
            }
            Action::Window(window_action) => {
                info!("Executing window action: {:?}", window_action.command);
                self.simulator.simulate_window_command(window_action)
            }
            Action::Run(run_action) => {
                info!("Executing run action: {} {:?}", run_action.command, run_action.args);
                self.simulator.run_program(run_action)
            }
        }
    }

    /// Execute an action by name (for testing)
    pub fn execute_by_name(&self, name: &str) -> Result<()> {
        info!("Executing action: {}", name);
        // This is a placeholder for testing
        Ok(())
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = CommandExecutor::new();
        assert!(executor.execute_by_name("test").is_ok());
    }
}
