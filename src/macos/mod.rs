//! macOS platform integration module
//!
//! Unified around MacOSSession (similar to ProtoolsSession pattern).
//! All macOS operations go through MacOSSession for consistency and resource management.

// Core infrastructure
pub mod ffi;
pub mod session;

// Stable modules
pub mod events;
pub mod notifications;
pub mod menu_bar;

// Session implementation modules (private)
mod app;
mod ui;
mod input;

// New menu and keystroke implementation (private - accessed via MacOSSession)
#[path = "menu_new.rs"]
mod menu_impl;
#[path = "keystroke_new.rs"]
mod keystroke_impl;

// Temporary: Keep old menu.rs for get_app_menus (used by ProTools plugin search)
// Will be removed or migrated in Phase 1
#[path = "menu.rs"]
mod menu_old;

// Commands and actions
pub mod commands;
pub mod actions;

// Re-export main session type
pub use session::MacOSSession;

// Re-export commonly used items
pub use events::*;
pub use notifications::*;

// Old modules (temporarily kept for reference, not imported)
// #[allow(dead_code)]
// mod app_info;
// #[allow(dead_code)]
// mod ui_elements;
// #[allow(dead_code)]
// mod input_dialog;

// ============================================================================
// Compatibility wrappers for old public API
// These create a MacOSSession and call the appropriate method
// This keeps existing code (protools, soundminer, hotkey) working
// ============================================================================

/// Compatibility wrapper for keystroke module
pub mod keystroke {
    use anyhow::Result;

    pub fn send_keystroke(keys: &[&str]) -> Result<()> {
        // Extract keycode and modifiers from keys array
        // For now, just fail - this needs proper key parsing
        anyhow::bail!("send_keystroke compatibility wrapper not fully implemented yet - use MacOSSession.send_keystroke() instead")
    }
}

/// Compatibility wrapper for menu module
pub mod menu {
    use anyhow::Result;

    // Re-export MenuBar type from old menu module
    pub use super::menu_old::{MenuBar, MenuItem};

    pub fn run_menu_item(app_name: &str, menu_path: &[&str]) -> Result<()> {
        let mut macos = super::MacOSSession::new()?;
        crate::async_runtime::runtime().block_on(macos.click_menu_item(app_name, menu_path))
    }

    pub fn get_app_menus(app_name: &str) -> Result<MenuBar> {
        // Temporary: Use old implementation
        // This will be migrated to MacOSSession in Phase 1
        super::menu_old::get_app_menus(app_name)
    }
}

/// Compatibility wrapper for app_info module
pub mod app_info {
    use anyhow::Result;

    pub fn get_current_app() -> Result<String> {
        let mut macos = super::MacOSSession::new()?;
        crate::async_runtime::runtime().block_on(macos.get_focused_app())
    }

    pub fn get_app_window() -> Result<String> {
        let mut macos = super::MacOSSession::new()?;
        crate::async_runtime::runtime().block_on(macos.get_focused_window())
    }

    pub fn has_accessibility_permission() -> bool {
        if let Ok(macos) = super::MacOSSession::new() {
            macos.has_accessibility_permission()
        } else {
            false
        }
    }

    pub fn focus_application(app_name: &str) -> Result<()> {
        let mut macos = super::MacOSSession::new()?;
        crate::async_runtime::runtime().block_on(macos.focus_app(app_name))
    }
}

/// Compatibility wrapper for ui_elements module
pub mod ui_elements {
    use anyhow::Result;

    pub fn click_button(app_name: &str, window_name: &str, button_name: &str) -> Result<()> {
        let mut macos = super::MacOSSession::new()?;
        crate::async_runtime::runtime().block_on(macos.click_button(app_name, window_name, button_name))
    }

    pub fn window_exists(app_name: &str, window_name: &str) -> Result<bool> {
        let mut macos = super::MacOSSession::new()?;
        crate::async_runtime::runtime().block_on(macos.window_exists(app_name, window_name))
    }

    pub fn wait_for_window(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<bool> {
        use std::time::{Duration, Instant};
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            if window_exists(app_name, window_name).unwrap_or(false) {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(false)
    }

    pub fn close_window_with_retry(app_name: &str, window_name: &str, max_retries: usize) -> Result<()> {
        let mut macos = super::MacOSSession::new()?;

        for attempt in 0..max_retries {
            match crate::async_runtime::runtime().block_on(macos.close_window(app_name, window_name)) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt == max_retries - 1 {
                        return Err(e);
                    }
                    log::warn!("Close window attempt {} failed: {}", attempt + 1, e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        anyhow::bail!("Failed to close window after {} retries", max_retries)
    }
}
