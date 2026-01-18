//! RustGesture - A modern mouse gesture software for Windows
//!
//! This library provides the core functionality for gesture recognition and execution.

pub mod config;
pub mod core;
pub mod winapi;

pub use config::{config::GestureConfig, manager::ConfigManager};
pub use core::gesture::{Gesture, GestureDir, GestureModifier, GestureTriggerButton, Point};
