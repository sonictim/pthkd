//! Application focus and window information
//!
//! Provides functions to get information about the currently focused application,
//! window, and UI elements.
//!
//! All functions now use the Swift bridge for native macOS integration.

use super::ffi::*;
use anyhow::Result;

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
/// Returns true if the focused element is a text field, text area, or similar editable text control.
/// This is useful for preventing hotkeys from triggering when the user is typing.
pub fn is_in_text_field() -> Result<bool> {
    Ok(crate::swift_bridge::is_in_text_field())
}
