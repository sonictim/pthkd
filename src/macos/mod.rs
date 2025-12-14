//! macOS platform integration module
//!
//! This module provides macOS-specific functionality organized by concern:
//! - `ffi`: Shared FFI declarations for macOS frameworks
//! - `events`: Core keyboard event tap (STABLE)
//! - `notifications`: System notifications (STABLE)
//! - `keystroke`: Sending keystrokes to apps (EXPERIMENTAL)
//! - `menu`: Clicking menu items (EXPERIMENTAL)
//! - `app_info`: Application focus and window information (EXPERIMENTAL)
//! - `actions`: macOS actions callable from config (namespace: "os")

// FFI declarations (shared across modules)
pub mod ffi;
pub mod helpers;

// Stable modules
pub mod events;
pub mod notifications;

// Experimental modules (work in progress)
pub mod keystroke;
pub mod menu;
pub mod app_info;
pub mod ui_elements;
pub mod input_dialog;

// Commands and Actions
pub mod commands;
pub mod actions;

// Re-export commonly used items
pub use events::*;
pub use notifications::*;

// Experimental items are not re-exported - must be explicitly imported
// This makes it clear when experimental code is being used
