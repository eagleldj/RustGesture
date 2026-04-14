//! Windows API bindings and utilities
//!
//! This module provides safe wrappers around Windows API calls for:
//! - Mouse/keyboard hooks
//! - Input simulation
//! - Window management
//! - System information

pub mod helpers;
pub mod hook;
pub mod input;
pub mod message_loop;
pub mod overlay;

// TODO: Implement these modules
// pub mod window;
