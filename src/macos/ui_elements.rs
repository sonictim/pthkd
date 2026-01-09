//! UI element automation for macOS
//!
//! Provides functions to find and interact with UI elements (buttons, etc.)
//! in application windows using the macOS Accessibility API.
//!
//! **Thread Safety**: All public functions automatically dispatch to the main
//! thread using Grand Central Dispatch, as required by the macOS Accessibility API.

use super::ffi::*;
use super::helpers::{CFArray, CFNumber};
use anyhow::{Context, Result, bail};
use libc::c_void;
use std::ptr;

// ============================================================================
// Thread Safety Helper
// ============================================================================

/// Helper to dispatch a closure to the main thread and wait for result
///
/// All Accessibility API calls must run on the main thread. This helper
/// uses Grand Central Dispatch to ensure thread safety.
unsafe fn dispatch_to_main<F, R>(f: F) -> Result<R>
where
    F: FnOnce() -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    unsafe {
        use std::sync::mpsc;
        use std::time::Duration;
        let (tx, rx) = mpsc::channel();

        super::events::dispatch_to_main_queue(move || {
            // Catch panics to prevent wedging the main queue
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

            let final_result = match result {
                Ok(r) => r,
                Err(panic_info) => {
                    log::error!("UI operation PANICKED: {:?}", panic_info);
                    Err(anyhow::anyhow!("UI operation panicked"))
                }
            };

            let _ = tx.send(final_result);
        });

        rx.recv_timeout(Duration::from_secs(5)).map_err(|e| {
            anyhow::anyhow!("UI operation timed out: {}. Main thread may be blocked.", e)
        })?
    }
}

/// Get all buttons in a window
///
/// **Thread Safety**: Automatically dispatches to main thread.
///
/// # Arguments
/// * `app_name` - Name of the application (e.g., "Pro Tools")
/// * `window_name` - Name of the window to search in, or "" for focused window
///
/// # Returns
/// Vector of button titles found in the window
///
/// # Example
/// ```ignore
/// let buttons = get_window_buttons("Pro Tools", "AudioSuite: Reverb")?;
/// // Returns: ["Preview", "Render", "Cancel"]
/// ```
pub fn get_window_buttons(app_name: &str, window_name: &str) -> Result<Vec<String>> {
    // Use Swift bridge instead of direct AX API to avoid crashes
    crate::swift_bridge::get_window_buttons(app_name, window_name)
}

/// Click a button in a window
///
/// # Arguments
/// * `app_name` - Name of the application (e.g., "Pro Tools")
/// * `window_name` - Name of the window, or "" for focused window
/// * `button_name` - Name of the button to click (soft matched)
///
/// # Example
/// ```ignore
/// // Click "Render" button in the focused window
/// click_button("Pro Tools", "", "Render")?;
///
/// // Click "OK" in a specific plugin window
/// click_button("Pro Tools", "AudioSuite: Reverb", "OK")?;
/// ```
pub fn click_button(app_name: &str, window_name: &str, button_name: &str) -> Result<()> {
    // Use Swift bridge instead of direct AX API to avoid crashes
    crate::swift_bridge::click_button(app_name, window_name, button_name)?;

    log::info!(
        "✅ Clicked button '{}' in window '{}' of app '{}'",
        button_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        },
        if app_name.is_empty() {
            "<frontmost>"
        } else {
            app_name
        }
    );

    Ok(())
}
pub fn click_checkbox(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    // Use Swift bridge instead of direct AX API to avoid crashes
    crate::swift_bridge::click_checkbox(app_name, window_name, checkbox_name)?;

    log::info!(
        "✅ Clicked checkbox '{}' in window '{}' of app '{}'",
        checkbox_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        },
        if app_name.is_empty() {
            "<frontmost>"
        } else {
            app_name
        }
    );

    Ok(())
}

/// Check a checkbox (set to checked state)
pub fn check_box(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    crate::swift_bridge::set_checkbox_value(app_name, window_name, checkbox_name, 1)?;
    log::info!(
        "✅ Set checkbox '{}' in window '{}' to CHECKED",
        checkbox_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        }
    );
    Ok(())
}

/// Get menu items from a popup button
pub fn get_popup_menu_items(
    app_name: &str,
    window_name: &str,
    popup_name: &str,
) -> Result<Vec<String>> {
    crate::swift_bridge::get_popup_menu_items(app_name, window_name, popup_name)
}

/// Uncheck a checkbox (set to unchecked state)
pub fn uncheck_box(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    crate::swift_bridge::set_checkbox_value(app_name, window_name, checkbox_name, 0)?;
    log::info!(
        "✅ Set checkbox '{}' in window '{}' to UNCHECKED",
        checkbox_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        }
    );
    Ok(())
}
// ============================================================================
// Helper Functions
// ============================================================================

/// Get the currently focused window
pub(crate) unsafe fn get_focused_window(app_element: AXUIElementRef) -> Result<AXUIElementRef> {
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        os.get_ax_element_attr(app_element, "AXFocusedWindow")
            .context("Failed to get focused window")
    }
}

/// Get all window titles for an application
///
/// Returns a vector of window titles in the order they appear in the window list
///
/// # Arguments
/// * `app_name` - Name of the application
///
/// # Example
/// ```ignore
/// let titles = get_window_titles("iZotope RX")?;
/// // Returns: ["iZotope RX 11 Audio Editor", "iZotope RX 11 Audio Editor"]
/// // (2 windows with same title means render dialog is open)
/// ```
pub fn get_window_titles(app_name: &str) -> Result<Vec<String>> {
    crate::swift_bridge::get_window_titles(app_name)
}

/// Find a window by name using soft matching
pub(crate) unsafe fn find_window_by_name(
    app_element: AXUIElementRef,
    window_name: &str,
) -> Result<AXUIElementRef> {
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        // Get all windows using building block
        let windows_value = os.get_ax_element_attr(app_element, "AXWindows")?;
        let windows = CFArray::new(windows_value);

        // Search for window with matching title
        for i in 0..windows.count() {
            let window = windows.get(i) as AXUIElementRef;

            if let Ok(title) = os.get_ax_string_attr(window, "AXTitle") {
                // Use soft_match from main.rs
                if crate::soft_match(&title, window_name) {
                    // Found it! Retain the window before returning (CFArrayGetValueAtIndex returns non-retained)
                    CFRetain(window);
                    return Ok(window);
                }
            }
        }

        bail!("Window '{}' not found", window_name)
    }
}

// All UI element operations now using Swift bridge:
// - crate::swift_bridge::click_button()
// - crate::swift_bridge::click_checkbox()
// - crate::swift_bridge::set_checkbox_value()
// - crate::swift_bridge::get_window_buttons()
// - crate::swift_bridge::get_popup_menu_items()
// - crate::swift_bridge::get_window_text()

///
/// Check if a window exists right now
///
/// # Returns
/// Ok(true) if window exists, Ok(false) if not found
pub fn window_exists(app_name: &str, window_name: &str) -> Result<bool> {
    crate::swift_bridge::window_exists(app_name, window_name)
}

/// Wait for a window to appear
///
/// Polls every 50ms until window is found or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for (soft matched)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_exists(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    log::info!(
        "Waiting for window '{}' in '{}' (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    if crate::swift_bridge::wait_for_window(
        app_name,
        window_name,
        crate::swift_bridge::WindowCondition::Exists,
        timeout_ms as i32,
    )? {
        log::info!("✅ Window '{}' appeared", window_name);
        Ok(())
    } else {
        bail!(
            "Timeout waiting for window '{}' ({}ms)",
            window_name,
            timeout_ms
        )
    }
}

/// Wait for a window to disappear/hide
///
/// Polls every 50ms until window is gone or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for to disappear (soft matched)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_closed(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    log::info!(
        "Waiting for window '{}' in '{}' to close (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    if crate::swift_bridge::wait_for_window(
        app_name,
        window_name,
        crate::swift_bridge::WindowCondition::Closed,
        timeout_ms as i32,
    )? {
        log::info!("✅ Window '{}' has closed", window_name);
        Ok(())
    } else {
        bail!(
            "Timeout waiting for window '{}' to close ({}ms)",
            window_name,
            timeout_ms
        )
    }
}

/// Wait for a window to become focused
///
/// Polls every 100ms until the specified window becomes the focused window or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for to be focused (soft matched).
///   If empty string, just waits for the app to be focused (any window)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_focused(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    if window_name.is_empty() {
        log::info!(
            "Waiting for '{}' to be focused (timeout: {}ms)",
            app_name,
            timeout_ms
        );
    } else {
        log::info!(
            "Waiting for window '{}' in '{}' to be focused (timeout: {}ms)",
            window_name,
            app_name,
            timeout_ms
        );
    }

    if crate::swift_bridge::wait_for_window(
        app_name,
        window_name,
        crate::swift_bridge::WindowCondition::Focused,
        timeout_ms as i32,
    )? {
        if window_name.is_empty() {
            log::info!("✅ '{}' is now focused", app_name);
        } else {
            log::info!("✅ Window '{}' is now focused", window_name);
        }
        Ok(())
    } else if window_name.is_empty() {
        bail!(
            "Timeout waiting for '{}' to be focused ({}ms)",
            app_name,
            timeout_ms
        )
    } else {
        bail!(
            "Timeout waiting for window '{}' to be focused ({}ms)",
            window_name,
            timeout_ms
        )
    }
}

pub fn close_window(app_name: &str, window_name: &str) -> Result<()> {
    crate::swift_bridge::close_window(app_name, window_name, None)?;
    log::info!(
        "✅ Closed window '{}'",
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        }
    );
    Ok(())
}
/// Close a window with retry - keeps trying until window disappears
///
/// Useful when window might not close immediately (e.g., during rendering)
/// Retries every 500ms until window is gone or timeout reached
/// Close window with retry - keeps trying until window is gone
pub fn close_window_with_retry(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    crate::swift_bridge::close_window(app_name, window_name, Some(timeout_ms as i32))?;
    log::info!("✅ Window '{}' closed", window_name);
    Ok(())
}

/// Get all text content from a window
///
/// Recursively searches for AXStaticText elements and collects their values
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of the window, or "" for focused window
///
/// # Returns
/// Vector of text strings found in the window
///
/// # Example
/// ```ignore
/// let text = get_window_text("Pro Tools", "AudioSuite: Reverb")?;
/// // Returns: ["Press RENDER to commit changes", "Render", "Cancel", ...]
/// ```
pub fn get_window_text(app_name: &str, window_name: &str) -> Result<Vec<String>> {
    crate::swift_bridge::get_window_text(app_name, window_name)
}
