//! Core gesture recognition module
//!
//! This module contains the gesture recognition engine, including:
//! - Path tracking
//! - Gesture parsing
//! - Intent matching
//! - Action execution

pub mod gesture;
pub mod parser;
pub mod tracker;
pub mod recognizer;
pub mod intent;
pub mod executor;
pub mod app;
pub mod hook_callback;
pub mod capture;
