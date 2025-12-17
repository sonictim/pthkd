//! Application focus and window information
//!
//! Provides functions to get information about the currently focused application,
//! window, and UI elements.
//!
//! **PERMISSIONS REQUIRED:**
//! - `get_app_window()` and `is_in_text_field()` require Accessibility permissions
//! - User must enable this in: System Preferences > Security & Privacy > Accessibility
//! - `get_current_app()` does NOT require special permissions

use super::ffi::*;
use anyhow::{Result, bail};
use libc::c_void;
use std::ptr;

// ============================================================================
// Public API
// ============================================================================

/// Get the name of the currently focused (frontmost) application
///
/// **Permissions:** None required
///
/// # Example
/// ```ignore
/// let app_name = get_current_app()?;
/// println!("Current app: {}", app_name); // "Pro Tools"
/// ```
pub fn get_current_app() -> Result<String> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    unsafe {
        // Get NSWorkspace class
        let workspace_class = class!(NSWorkspace);

        // Call [NSWorkspace sharedWorkspace]
        let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
        if workspace.is_null() {
            bail!("Failed to get NSWorkspace");
        }

        // Call [workspace frontmostApplication]
        let app: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            bail!("No frontmost application");
        }

        // Call [app localizedName] - returns NSString*
        let name_nsstring: *mut AnyObject = msg_send![app, localizedName];
        if name_nsstring.is_null() {
            bail!("Could not get application name");
        }

        // Convert NSString to Rust String using CFString functions
        let name = cfstring_to_string(name_nsstring as *mut c_void)
            .unwrap_or_else(|| String::from("Unknown"));

        Ok(name)
    }
}

/// Get the title of the currently focused window
///
/// **Permissions:** Requires Accessibility permissions
/// - Enable in: System Preferences > Security & Privacy > Accessibility
///
/// # Example
/// ```ignore
/// let window_title = get_app_window()?;
/// println!("Window: {}", window_title); // "My Session - Pro Tools"
/// ```
pub fn get_app_window() -> Result<String> {
    unsafe {
        // Check accessibility permissions first
        if !AXIsProcessTrusted() {
            bail!(
                "Accessibility permissions not granted. \
                Enable in System Preferences > Security & Privacy > Accessibility"
            );
        }

        // Get frontmost app using NSWorkspace
        use objc2::runtime::AnyObject;
        use objc2::{class, msg_send};

        // Get NSWorkspace class
        let workspace_class = class!(NSWorkspace);

        // Call [NSWorkspace sharedWorkspace]
        let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
        if workspace.is_null() {
            bail!("Failed to get NSWorkspace");
        }

        // Call [workspace frontmostApplication]
        let app: *mut AnyObject = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            bail!("No frontmost application");
        }

        // Call [app processIdentifier]
        let pid: i32 = msg_send![app, processIdentifier];

        // Create accessibility element for the app
        let app_element = AXUIElementCreateApplication(pid);
        if app_element.is_null() {
            bail!("Failed to create accessibility element for app");
        }

        // Get the focused window
        let focused_window_attr = create_cfstring("AXFocusedWindow");
        let mut focused_window: *mut c_void = ptr::null_mut();

        let result =
            AXUIElementCopyAttributeValue(app_element, focused_window_attr, &mut focused_window);

        CFRelease(focused_window_attr);

        if result != K_AX_ERROR_SUCCESS {
            CFRelease(app_element);

            if result == K_AX_ERROR_NO_VALUE {
                bail!("No focused window (app may not have windows)");
            } else if result == K_AX_ERROR_API_DISABLED {
                bail!("Accessibility API disabled");
            } else {
                bail!("Failed to get focused window (error code: {})", result);
            }
        }

        if focused_window.is_null() {
            CFRelease(app_element);
            bail!("No focused window");
        }

        // Get the window title
        let title_attr = create_cfstring("AXTitle");
        let mut title_cfstring: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(focused_window, title_attr, &mut title_cfstring);

        CFRelease(title_attr);
        CFRelease(focused_window);
        CFRelease(app_element);

        if result != K_AX_ERROR_SUCCESS {
            if result == K_AX_ERROR_NO_VALUE {
                // Window exists but has no title (common for some windows)
                return Ok(String::from("(Untitled)"));
            } else {
                bail!("Failed to get window title (error code: {})", result);
            }
        }

        // Convert CFString to Rust String
        let title =
            cfstring_to_string(title_cfstring).unwrap_or_else(|| String::from("(Untitled)"));

        if !title_cfstring.is_null() {
            CFRelease(title_cfstring);
        }

        Ok(title)
    }
}

/// Check if the user is currently focused in a text entry field
///
/// **Permissions:** Requires Accessibility permissions
/// - Enable in: System Preferences > Security & Privacy > Accessibility
///
/// **Note:** This is "best effort" - works reliably in native macOS apps,
/// but may not work correctly in Electron apps, browsers, or apps that don't
/// properly expose accessibility information.
///
/// # Example
/// ```ignore
/// if is_in_text_field()? {
///     println!("User is typing in a text field");
/// }
/// ```
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

        // Convert role to string and check if it's a text field
        let role = cfstring_to_string(role_cfstring).unwrap_or_default();

        if !role_cfstring.is_null() {
            CFRelease(role_cfstring);
        }

        // Log the role for debugging
        log::info!("Focused element role: '{}'", role);

        // Check for common text field roles
        let is_text = role == "AXTextField"
            || role == "AXTextArea"
            || role == "AXComboBox"
            || role == "AXSearchField"
            || role == "AXStaticText"; // Sometimes editable text shows as this

        Ok(is_text)
    }
}

/// Check if accessibility permissions are granted
///
/// Returns `true` if the app has been granted accessibility permissions,
/// `false` otherwise.
pub fn has_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Get a list of all currently running applications
///
/// **Permissions:** None required
///
/// # Returns
/// A vector of application names (e.g., ["Pro Tools", "Safari", "Finder"])
///
/// # Example
/// ```ignore
/// let apps = get_all_running_applications()?;
/// for app in apps {
///     println!("Running: {}", app);
/// }
/// ```
pub fn get_all_running_applications() -> Result<Vec<String>> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    unsafe {
        // Get NSWorkspace class
        let workspace_class = class!(NSWorkspace);

        // Call [NSWorkspace sharedWorkspace]
        let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
        if workspace.is_null() {
            bail!("Failed to get NSWorkspace");
        }

        // Call [workspace runningApplications] to get array of all running apps
        let running_apps: *mut AnyObject = msg_send![workspace, runningApplications];
        if running_apps.is_null() {
            bail!("Failed to get running applications");
        }

        // Get the count of running applications
        let count: usize = msg_send![running_apps, count];

        let mut app_names = Vec::new();

        // Iterate through running apps and collect names
        for i in 0..count {
            let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
            if app.is_null() {
                continue;
            }

            // Get the localized name of this app
            let name_nsstring: *mut c_void = msg_send![app, localizedName];
            if name_nsstring.is_null() {
                continue;
            }

            // Convert to Rust string
            if let Some(name) = cfstring_to_string(name_nsstring)
                && !name.is_empty()
            {
                app_names.push(name);
            }
        }

        Ok(app_names)
    }
}

/// Bring a specific application to the foreground by name
///
/// **Permissions:** None required
///
/// Matching strategy (case-insensitive):
/// 1. First tries exact match (e.g., "pro tools" matches "Pro Tools")
/// 2. Falls back to contains() if no exact match (e.g., "pro" matches "Pro Tools")
///
/// # Arguments
/// * `app_name` - Application name to match (case-insensitive)
///
/// # Example
/// ```ignore
/// focus_application("Pro Tools")?;  // Exact match preferred
/// focus_application("pro tools")?;  // Case-insensitive
/// focus_application("Pro")?;         // Falls back to contains()
/// ```
pub fn focus_application(app_name: &str) -> Result<()> {
    use objc2::msg_send;

    unsafe {
        super::helpers::with_running_app(app_name, |app| {
            // NSApplicationActivateAllWindows = 1 << 0 = 1
            // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
            // Combine both flags: 1 | 2 = 3
            let options: usize = 3;
            let success: bool = msg_send![app, activateWithOptions: options];

            if success {
                log::info!("Successfully activated application: {}", app_name);
                Ok(())
            } else {
                bail!("Failed to activate application: {}", app_name);
            }
        })
    }
}

/// Get the PID (process ID) of an application by name
///
/// Uses soft_match for case-insensitive, whitespace-insensitive matching
pub fn get_pid_by_name(app_name: &str) -> Result<i32> {
    use objc2::msg_send;

    unsafe {
        super::helpers::with_running_app(app_name, |app| {
            let pid: i32 = msg_send![app, processIdentifier];
            Ok(pid)
        })
    }
}

/// Launch an application by name
///
/// **Permissions:** None required
///
/// # Arguments
/// * `app_name` - Name of the application to launch (case-insensitive)
///
/// # Example
/// ```ignore
/// launch_application("Pro Tools")?;
/// ```
pub fn launch_application(app_name: &str) -> Result<()> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    log::warn!(
        "⚠️  launch_application called with app_name: '{}'",
        app_name
    );
    log::warn!(
        "⚠️  Stack trace: {:?}",
        std::backtrace::Backtrace::capture()
    );

    unsafe {
        let workspace_class = class!(NSWorkspace);
        let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
        if workspace.is_null() {
            bail!("Failed to get NSWorkspace");
        }

        // Create NSString for the app name
        let app_name_cfstring = create_cfstring(app_name);

        // Launch the application
        let success: bool = msg_send![workspace, launchApplication: app_name_cfstring];

        CFRelease(app_name_cfstring);

        if success {
            log::info!("Successfully launched application: {}", app_name);
            Ok(())
        } else {
            bail!("Failed to launch application: {}", app_name)
        }
    }
}

/// Check if the current focused app has a visible/focused window
///
/// Returns true if there's a focused window, false if windows are hidden
pub fn has_focused_window() -> bool {
    get_app_window().is_ok()
}

/// Wait for the focused app to have no visible windows (all windows hidden)
///
/// Useful for waiting for apps like RX that hide their window after Cmd+Enter
///
/// # Arguments
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_windows_to_hide(timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(50);

    println!("⏳ Waiting for windows to hide (timeout: {}ms)", timeout_ms);
    log::info!("Waiting for windows to hide (timeout: {}ms)", timeout_ms);

    loop {
        if let Ok(window) = get_app_window() {
            println!("⏳ Window still visible: '{}', waiting...", window);
        } else {
            println!("✅ All windows are now hidden");
            log::info!("✅ All windows are now hidden");
            return Ok(());
        }

        if start.elapsed() >= timeout {
            println!("❌ Timeout waiting for windows to hide ({}ms)", timeout_ms);
            bail!("Timeout waiting for windows to hide ({}ms)", timeout_ms);
        }

        std::thread::sleep(poll_interval);
    }
}

/// Wait for an app to no longer be focused
///
/// Polls every 50ms until the app is no longer the frontmost app
///
/// # Arguments
/// * `app_name` - Name of the application to wait for to lose focus
/// * `timeout_ms` - Maximum time to wait in milliseconds
///
/// # Example
/// ```ignore
/// // Wait for RX to lose focus (e.g., after Cmd+Enter sends it to background)
/// wait_for_app_to_lose_focus("RX 11", 10000)?;
/// ```
pub fn wait_for_app_to_lose_focus(app_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(50);

    println!(
        "⏳ Waiting for '{}' to lose focus (timeout: {}ms)",
        app_name, timeout_ms
    );
    log::info!(
        "Waiting for '{}' to lose focus (timeout: {}ms)",
        app_name,
        timeout_ms
    );

    loop {
        if let Ok(current_app) = get_current_app() {
            println!("🔍 Current focused app: '{}'", current_app);

            if !crate::soft_match(&current_app, app_name) {
                println!("✅ '{}' is no longer focused", app_name);
                log::info!("✅ '{}' is no longer focused", app_name);
                return Ok(());
            } else {
                println!("⏳ '{}' is still focused, waiting...", app_name);
            }
        }

        if start.elapsed() >= timeout {
            println!(
                "❌ Timeout waiting for '{}' to lose focus ({}ms)",
                app_name, timeout_ms
            );
            bail!(
                "Timeout waiting for '{}' to lose focus ({}ms)",
                app_name,
                timeout_ms
            );
        }

        std::thread::sleep(poll_interval);
    }
}

/// Focus an application, optionally switching and/or launching, and wait for confirmation
///
/// This is a robust all-in-one function that:
/// 1. If `switch` is true, tries to switch to the application
/// 2. If switch fails (or is disabled) and `launch` is true, launches the app
/// 3. Polls to confirm the app (and optionally specific window) is actually focused
///
/// **Permissions:** None required (but window checking requires Accessibility permissions)
///
/// # Arguments
/// * `app_name` - Name of the application (case-insensitive)
/// * `window_name` - Name of specific window to wait for, or "" for any window
/// * `switch` - If true, attempts to switch to app if already running
/// * `launch` - If true, attempts to launch app if not running or switch fails
/// * `timeout_ms` - Maximum time to wait for focus in milliseconds
///
/// # Example
/// ```ignore
/// // Focus Pro Tools, switch and launch if needed, any window
/// focus_app("Pro Tools", "", true, true, 5000)?;
///
/// // Focus existing app only (switch but don't launch), wait for specific window
/// focus_app("Pro Tools", "Edit", true, false, 1000)?;
/// ```
pub fn focus_app(
    app_name: &str,
    window_name: &str,
    switch: bool, // Try to switch to app if already running
    launch: bool, // Try to launch app if not running or switch fails
    timeout_ms: u64,
) -> Result<()> {
    use std::time::{Duration, Instant};

    if window_name.is_empty() {
        println!("🎯 Attempting to focus '{}'...", app_name);
    } else {
        println!(
            "🎯 Attempting to focus '{}' (window: '{}')...",
            app_name, window_name
        );
    }

    // Try to switch to the application if already running
    if switch {
        println!("🔄 Attempting to switch to '{}'...", app_name);
        if let Err(e) = focus_application(app_name) {
            println!("⚠️  Switch failed: {}", e);

            // If switch failed and launch is enabled, try launching
            if launch {
                println!("🚀 Launching '{}'...", app_name);
                launch_application(app_name)?;
            }
        } else {
            println!("✅ Switch successful");
        }
    } else if launch {
        // If switch is disabled but launch is enabled, just launch
        println!("🚀 Launching '{}'...", app_name);
        launch_application(app_name)?;
    }

    // Now poll to confirm the app is focused
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(50);

    loop {
        if let Ok(current_app) = get_current_app()
            && crate::soft_match(&current_app, app_name)
        {
            // App is focused
            if window_name.is_empty() {
                // If no specific window requested, just app focus is enough
                println!(
                    "✅ '{}' is now focused (no specific window required)",
                    app_name
                );
                log::info!("✅ '{}' is now focused", app_name);
                return Ok(());
            }

            // Check if the right window is focused
            if let Ok(current_window) = get_app_window()
                && crate::soft_match(&current_window, window_name)
            {
                println!("✅ Window '{}' is now focused", window_name);
                log::info!("✅ Window '{}' is now focused", window_name);
                return Ok(());
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
