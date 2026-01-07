//! Application focus and window information
//!
//! Provides functions to get information about the currently focused application,
//! window, and UI elements.
//!
//! All functions now use the Swift bridge for native macOS integration.

use anyhow::{Result, bail};
use super::ffi::*;
use libc::c_void;
use std::ptr;

/// Get the name of the currently focused (frontmost) application
///
/// # Example
/// ```ignore
/// let app_name = get_current_app()?;
/// println!("Current app: {}", app_name); // "Pro Tools"
/// ```
pub fn get_current_app() -> Result<String> {
    let info = crate::swift_bridge::get_frontmost_info()?;
    Ok(info.app)
}

/// Get the title of the currently focused window
///
/// # Example
/// ```ignore
/// let window_title = get_app_window()?;
/// println!("Window: {}", window_title); // "My Session - Pro Tools"
/// ```
pub fn get_app_window() -> Result<String> {
    let info = crate::swift_bridge::get_frontmost_info()?;
    Ok(info.window)
}

/// Focus/activate an application (simple version)
///
/// # Arguments
/// * `app_name` - Name of the application to focus
pub fn focus_application(app_name: &str) -> Result<()> {
    crate::swift_bridge::focus_app(app_name, "", true, false, 1000)
}

/// Focus/activate an application with options
///
/// # Arguments
/// * `app_name` - Name of app to focus
/// * `window_name` - Specific window to wait for (empty = any window)
/// * `should_switch` - Whether to switch to the app
/// * `should_launch` - Whether to launch if not running
/// * `timeout` - Maximum time to wait in milliseconds
pub fn focus_app(
    app_name: &str,
    window_name: &str,
    should_switch: bool,
    should_launch: bool,
    timeout: i32,
) -> Result<()> {
    crate::swift_bridge::focus_app(app_name, window_name, should_switch, should_launch, timeout)
}

/// Launch an application
///
/// # Arguments
/// * `app_name` - Name of the application to launch
pub fn launch_application(app_name: &str) -> Result<()> {
    crate::swift_bridge::launch_app(app_name)
}

/// Get list of all running application names
pub fn get_running_apps() -> Result<Vec<String>> {
    crate::swift_bridge::get_running_apps()
}

/// Get list of all running application names (alias for compatibility)
pub fn get_all_running_applications() -> Result<Vec<String>> {
    get_running_apps()
}

/// Check if the process has accessibility permissions
///
/// Returns true if accessibility permissions are granted, false otherwise
pub fn has_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Get the process ID (PID) for an application by name
///
/// # Arguments
/// * `app_name` - Name of the application
pub fn get_pid_by_name(app_name: &str) -> Result<i32> {
    use objc2::msg_send;

    unsafe {
        super::helpers::with_running_app(app_name, |app| {
            let pid: i32 = msg_send![app, processIdentifier];
            Ok(pid)
        })
    }
}

/// Check if the currently focused UI element is a text field
///
/// Returns true if the focused element is a text field, text area, or similar editable text control
pub fn is_in_text_field() -> Result<bool> {
    unsafe {
        // Check accessibility permissions first
        if !AXIsProcessTrusted() {
            bail!(
                "Accessibility permissions not granted. \
                Enable in System Preferences > Security & Privacy > Accessibility"
            );
        }

        // Get system-wide focused element
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            bail!("Failed to create system-wide accessibility element");
        }

        // Get the currently focused UI element
        let focused_element_attr = create_cfstring("AXFocusedUIElement");
        let mut focused_element: *mut c_void = ptr::null_mut();

        let result =
            AXUIElementCopyAttributeValue(system_wide, focused_element_attr, &mut focused_element);

        CFRelease(focused_element_attr);
        CFRelease(system_wide);

        if result != K_AX_ERROR_SUCCESS {
            if result == K_AX_ERROR_NO_VALUE {
                // No element focused - definitely not a text field
                return Ok(false);
            } else if result == K_AX_ERROR_INVALID_UI_ELEMENT {
                // Invalid element - this can happen in some apps
                log::debug!("Invalid UI element - assuming not a text field");
                return Ok(false);
            } else if result == K_AX_ERROR_API_DISABLED {
                bail!("Accessibility API disabled");
            } else {
                log::warn!(
                    "Failed to get focused element (error code: {}), assuming not a text field",
                    result
                );
                return Ok(false);
            }
        }

        if focused_element.is_null() {
            return Ok(false);
        }

        // Get the role of the focused element
        let role_attr = create_cfstring("AXRole");
        let mut role_cfstring: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(focused_element, role_attr, &mut role_cfstring);

        CFRelease(role_attr);
        CFRelease(focused_element);

        if result != K_AX_ERROR_SUCCESS {
            // If we can't get the role, assume it's not a text field
            return Ok(false);
        }

        if role_cfstring.is_null() {
            return Ok(false);
        }

        // Convert CFString to Rust String
        let role_cfstring = role_cfstring as CFStringRef;
        let role_string = cfstring_to_string(role_cfstring)
            .ok_or_else(|| anyhow::anyhow!("Failed to convert role CFString to Rust String"))?;

        CFRelease(role_cfstring);

        // Check if the role indicates a text input field
        let is_text = role_string == "AXTextField"
            || role_string == "AXTextArea"
            || role_string == "AXComboBox"
            || role_string == "AXSearchField";

        Ok(is_text)
    }
}
