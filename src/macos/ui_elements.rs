//! UI element automation for macOS
//!
//! Provides functions to find and interact with UI elements (buttons, etc.)
//! in application windows using the macOS Accessibility API.

use super::app_info::get_pid_by_name;
use super::ffi::*;
use anyhow::{Context, Result, bail};
use libc::c_void;
use std::ptr;

/// Get all buttons in a window
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
    unsafe {
        let pid = get_pid_by_name(app_name).context(format!("Failed to find app: {}", app_name))?;

        let app_element = AXUIElementCreateApplication(pid);

        // Get the target window (focused or by name)
        let window = if window_name.is_empty() {
            get_focused_window(app_element)?
        } else {
            find_window_by_name(app_element, window_name)?
        };

        // Get all buttons in the window
        let buttons = find_buttons_in_element(window)?;

        // Clean up
        CFRelease(app_element);
        CFRelease(window);

        Ok(buttons)
    }
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
    unsafe {
        let pid = get_pid_by_name(app_name).context(format!("Failed to find app: {}", app_name))?;

        let app_element = AXUIElementCreateApplication(pid);

        // Get the target window
        let window = if window_name.is_empty() {
            get_focused_window(app_element)?
        } else {
            find_window_by_name(app_element, window_name)?
        };

        // Find the button
        let button_element = find_button_in_window(window, button_name)?;

        // Click it
        let press_action = create_cfstring("AXPress");
        let result = AXUIElementPerformAction(button_element, press_action);
        CFRelease(press_action);

        if result != K_AX_ERROR_SUCCESS {
            bail!(
                "Failed to press button '{}' (error code: {})",
                button_name,
                result
            );
        }

        // Clean up
        CFRelease(button_element);
        CFRelease(window);
        CFRelease(app_element);

        log::info!(
            "✅ Clicked button '{}' in window '{}'",
            button_name,
            if window_name.is_empty() {
                "<focused>"
            } else {
                window_name
            }
        );

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the currently focused window
unsafe fn get_focused_window(app_element: AXUIElementRef) -> Result<AXUIElementRef> {
    let attr = create_cfstring("AXFocusedWindow");
    let mut window: *mut c_void = ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(app_element, attr, &mut window);
    CFRelease(attr);

    if result != K_AX_ERROR_SUCCESS {
        bail!("Failed to get focused window (error: {})", result);
    }

    if window.is_null() {
        bail!("No focused window");
    }

    Ok(window)
}

/// Find a window by name using soft matching
unsafe fn find_window_by_name(
    app_element: AXUIElementRef,
    window_name: &str,
) -> Result<AXUIElementRef> {
    // Get all windows
    let windows_attr = create_cfstring("AXWindows");
    let mut windows_value: *mut c_void = ptr::null_mut();

    let result = AXUIElementCopyAttributeValue(app_element, windows_attr, &mut windows_value);
    CFRelease(windows_attr);

    if result != K_AX_ERROR_SUCCESS || windows_value.is_null() {
        bail!("Failed to get windows list");
    }

    let windows_count = CFArrayGetCount(windows_value);

    // Search for window with matching title
    for i in 0..windows_count {
        let window = CFArrayGetValueAtIndex(windows_value, i) as AXUIElementRef;

        let title_attr = create_cfstring("AXTitle");
        let mut title_value: *mut c_void = ptr::null_mut();

        AXUIElementCopyAttributeValue(window, title_attr, &mut title_value);
        CFRelease(title_attr);

        if let Some(title) = cfstring_to_string(title_value) {
            if !title_value.is_null() {
                CFRelease(title_value);
            }

            // Use soft_match from main.rs
            if crate::soft_match(&title, window_name) {
                // Found it! Return this window
                return Ok(window);
            }
        }
    }

    bail!("Window '{}' not found", window_name)
}

/// Find all buttons in a UI element (recursive)
unsafe fn find_buttons_in_element(element: AXUIElementRef) -> Result<Vec<String>> {
    let mut buttons = Vec::new();

    // Get role
    let role_attr = create_cfstring("AXRole");
    let mut role_value: *mut c_void = ptr::null_mut();
    AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
    CFRelease(role_attr);

    if let Some(role) = cfstring_to_string(role_value) {
        if !role_value.is_null() {
            CFRelease(role_value);
        }

        // If this is a button, get its title
        if role == "AXButton" {
            let title_attr = create_cfstring("AXTitle");
            let mut title_value: *mut c_void = ptr::null_mut();
            AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
            CFRelease(title_attr);

            if let Some(title) = cfstring_to_string(title_value) {
                if !title_value.is_null() {
                    CFRelease(title_value);
                }
                if !title.is_empty() {
                    buttons.push(title);
                }
            }
        }
    }

    // Recursively search children
    let children_attr = create_cfstring("AXChildren");
    let mut children_value: *mut c_void = ptr::null_mut();
    let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
    CFRelease(children_attr);

    if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
        let children_count = CFArrayGetCount(children_value);

        for i in 0..children_count {
            let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
            if let Ok(mut child_buttons) = find_buttons_in_element(child) {
                buttons.append(&mut child_buttons);
            }
        }
    }

    Ok(buttons)
}

/// Find a button in a window by name (soft matched)
unsafe fn find_button_in_window(
    window: AXUIElementRef,
    button_name: &str,
) -> Result<AXUIElementRef> {
    find_button_in_element(window, button_name)
        .with_context(|| format!("Button '{}' not found in window", button_name))
}

/// Recursively find a button by name
unsafe fn find_button_in_element(
    element: AXUIElementRef,
    button_name: &str,
) -> Result<AXUIElementRef> {
    // Check if this element is a button with matching name
    let role_attr = create_cfstring("AXRole");
    let mut role_value: *mut c_void = ptr::null_mut();
    AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
    CFRelease(role_attr);

    if let Some(role) = cfstring_to_string(role_value) {
        if !role_value.is_null() {
            CFRelease(role_value);
        }

        if role == "AXButton" {
            let title_attr = create_cfstring("AXTitle");
            let mut title_value: *mut c_void = ptr::null_mut();
            AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
            CFRelease(title_attr);

            if let Some(title) = cfstring_to_string(title_value) {
                if !title_value.is_null() {
                    CFRelease(title_value);
                }

                if crate::soft_match(&title, button_name) {
                    // Found it! Return a copy
                    return Ok(element);
                }
            }
        }
    }

    // Search children recursively
    let children_attr = create_cfstring("AXChildren");
    let mut children_value: *mut c_void = ptr::null_mut();
    let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
    CFRelease(children_attr);

    if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
        let children_count = CFArrayGetCount(children_value);

        for i in 0..children_count {
            let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
            if let Ok(button) = find_button_in_element(child, button_name) {
                return Ok(button);
            }
        }
    }

    bail!("Button not found")
}
///
/// Check if a window exists right now
///
/// # Returns
/// Ok(true) if window exists, Ok(false) if not found
pub fn window_exists(app_name: &str, window_name: &str) -> Result<bool> {
    unsafe {
        let pid = get_pid_by_name(app_name)?;
        let app_element = AXUIElementCreateApplication(pid);

        let exists = find_window_by_name(app_element, window_name).is_ok();

        CFRelease(app_element);

        Ok(exists)
    }
}

/// Wait for a window to appear
///
/// Polls every 100ms until window is found or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for (soft matched)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(100);

    log::info!(
        "Waiting for window '{}' in '{}' (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    loop {
        if window_exists(app_name, window_name)? {
            log::info!("✅ Window '{}' appeared", window_name);
            return Ok(());
        }

        if start.elapsed() >= timeout {
            bail!(
                "Timeout waiting for window '{}' ({}ms)",
                window_name,
                timeout_ms
            );
        }

        std::thread::sleep(poll_interval);
    }
}

/// Wait for a window to become focused
///
/// Polls every 100ms until the specified window becomes the focused window or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for to be focused (soft matched).
///                   If empty string, just waits for the app to be focused (any window)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_focused(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(100);

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

    loop {
        // Check if the specified app is the current app
        if let Ok(current_app) = crate::macos::app_info::get_current_app() {
            if crate::soft_match(&current_app, app_name) {
                // App is focused
                if window_name.is_empty() {
                    // If no specific window requested, just app focus is enough
                    log::info!("✅ '{}' is now focused", app_name);
                    return Ok(());
                }

                // Check if the right window is focused
                if let Ok(current_window) = crate::macos::app_info::get_app_window() {
                    if crate::soft_match(&current_window, window_name) {
                        log::info!("✅ Window '{}' is now focused", window_name);
                        return Ok(());
                    }
                }
            }
        }

        if start.elapsed() >= timeout {
            if window_name.is_empty() {
                bail!(
                    "Timeout waiting for '{}' to be focused ({}ms)",
                    app_name,
                    timeout_ms
                );
            } else {
                bail!(
                    "Timeout waiting for window '{}' to be focused ({}ms)",
                    window_name,
                    timeout_ms
                );
            }
        }

        std::thread::sleep(poll_interval);
    }
}

pub fn close_window(app_name: &str, window_name: &str) -> Result<()> {
    unsafe {
        let pid = get_pid_by_name(app_name)?;
        let app_element = AXUIElementCreateApplication(pid);

        let window = if window_name.is_empty() {
            get_focused_window(app_element)?
        } else {
            find_window_by_name(app_element, window_name)?
        };

        // Get the close button (standard AXCloseButton)
        let close_attr = create_cfstring("AXCloseButton");
        let mut close_button: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(window, close_attr, &mut close_button);
        CFRelease(close_attr);

        if result != K_AX_ERROR_SUCCESS || close_button.is_null() {
            CFRelease(window);
            CFRelease(app_element);
            bail!("Window does not have a close button");
        }

        // Click the close button
        let press_action = create_cfstring("AXPress");
        let result = AXUIElementPerformAction(close_button, press_action);
        CFRelease(press_action);
        CFRelease(close_button);
        CFRelease(window);
        CFRelease(app_element);

        if result != K_AX_ERROR_SUCCESS {
            bail!("Failed to click close button");
        }

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
}
/// Close a window with retry - keeps trying until window disappears
///
/// Useful when window might not close immediately (e.g., during rendering)
/// Retries every 500ms until window is gone or timeout reached
/// Close window with retry - keeps trying until window is gone
pub fn close_window_with_retry(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    loop {
        // Check if already closed
        if !window_exists(app_name, window_name)? {
            log::info!("✅ Window '{}' closed", window_name);
            return Ok(());
        }

        // Try to close it
        let _ = close_window(app_name, window_name); // Ignore errors, will retry

        // Wait a bit before checking again
        std::thread::sleep(Duration::from_millis(100));

        if start.elapsed() >= timeout {
            bail!(
                "Timeout trying to close window '{}' ({}ms)",
                window_name,
                timeout_ms
            );
        }
    }
}
