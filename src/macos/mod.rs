//! macOS platform integration module
//!
//! This module provides macOS-specific functionality organized by concern:
//! - `ffi`: Shared FFI declarations for macOS frameworks
//! - `events`: Core keyboard event tap (STABLE)
//! - `notifications`: System notifications (STABLE)
//! - `keystroke`: Sending keystrokes to apps (EXPERIMENTAL)
//! - `app_info`: Application focus and window information (EXPERIMENTAL)
//! - `actions`: macOS actions callable from config (namespace: "os")
pub mod swift_bridge;
// FFI declarations (shared across modules)
pub mod ffi;
pub mod helpers;

// Stable modules
pub mod events;
pub mod notifications;
pub mod permissions;

// Core abstractions
pub mod session;

// Experimental modules (work in progress)
pub mod carbon_hotkeys;
pub mod keyring;
pub mod menubar;
pub mod window;

// Commands and Actions
pub mod actions;
pub mod commands;

// Re-export commonly used items
pub use events::*;
pub use notifications::*;
pub use session::MacOSSession;
pub use swift_bridge::*;
// Experimental items are not re-exported - must be explicitly imported
// This makes it clear when experimental code is being used
