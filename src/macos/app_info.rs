//! Application focus and window information
//!
//! Provides functions to get information about the currently focused application,
//! window, and UI elements.
//!
//! **PERMISSIONS REQUIRED:**
//! - `get_app_window()` and `is_in_text_field()` require Accessibility permissions
//! - User must enable this in: System Preferences > Security & Privacy > Accessibility
//! - `get_current_app()` does NOT require special permissions

use anyhow::{Result, bail};
use libc::c_void;
use std::ptr;

// ============================================================================
// Accessibility Framework FFI
// ============================================================================

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    // AXUIElement functions
    fn AXUIElementCreateApplication(pid: i32) -> *mut c_void;
    fn AXUIElementCreateSystemWide() -> *mut c_void;
    fn AXUIElementCopyAttributeValue(
        element: *mut c_void,
        attribute: *mut c_void,
        value: *mut *mut c_void,
    ) -> i32;

    // Accessibility permission check
    fn AXIsProcessTrusted() -> bool;
}

// Core Foundation functions (for CFString handling)
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *mut c_void);
    fn CFStringCreateWithCString(
        alloc: *mut c_void,
        c_str: *const i8,
        encoding: u32,
    ) -> *mut c_void;
    fn CFStringGetCString(
        the_string: *mut c_void,
        buffer: *mut u8,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
    fn CFStringGetLength(the_string: *mut c_void) -> isize;
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

// Accessibility error codes
const K_AX_ERROR_SUCCESS: i32 = 0;
const K_AX_ERROR_INVALID_UI_ELEMENT: i32 = -25204;
const K_AX_ERROR_API_DISABLED: i32 = -25211;
const K_AX_ERROR_NO_VALUE: i32 = -25212;

/// Helper to create a CFString from a Rust &str
unsafe fn create_cfstring(s: &str) -> *mut c_void {
    let c_str = std::ffi::CString::new(s).unwrap();
    unsafe { CFStringCreateWithCString(ptr::null_mut(), c_str.as_ptr(), K_CF_STRING_ENCODING_UTF8) }
}

/// Helper to convert CFString to Rust String
unsafe fn cfstring_to_string(cfstring: *mut c_void) -> Option<String> {
    if cfstring.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(cfstring) };
    if length == 0 {
        return Some(String::new());
    }

    // Allocate buffer with extra space for null terminator
    let buffer_size = (length * 4 + 1) as usize; // UTF-8 can be up to 4 bytes per char
    let mut buffer = vec![0u8; buffer_size];

    let success = unsafe {
        CFStringGetCString(
            cfstring,
            buffer.as_mut_ptr(),
            buffer_size as isize,
            K_CF_STRING_ENCODING_UTF8,
        )
    };

    if success {
        // Find the null terminator and create string from bytes
        let null_pos = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        String::from_utf8(buffer[..null_pos].to_vec()).ok()
    } else {
        None
    }
}

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
        let name_nsstring: *mut c_void = msg_send![app, localizedName];
        if name_nsstring.is_null() {
            bail!("Could not get application name");
        }

        // Convert NSString to Rust String using CFString functions
        let name = cfstring_to_string(name_nsstring).unwrap_or_else(|| String::from("Unknown"));

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

        let result = AXUIElementCopyAttributeValue(
            app_element,
            focused_window_attr,
            &mut focused_window,
        );

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

        let result = AXUIElementCopyAttributeValue(
            focused_window,
            title_attr,
            &mut title_cfstring,
        );

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
        let title = cfstring_to_string(title_cfstring).unwrap_or_else(|| String::from("(Untitled)"));

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

        let result = AXUIElementCopyAttributeValue(
            system_wide,
            focused_element_attr,
            &mut focused_element,
        );

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
                log::warn!("Failed to get focused element (error code: {}), assuming not a text field", result);
                return Ok(false);
            }
        }

        if focused_element.is_null() {
            return Ok(false);
        }

        // Get the role of the focused element
        let role_attr = create_cfstring("AXRole");
        let mut role_cfstring: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            focused_element,
            role_attr,
            &mut role_cfstring,
        );

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
            || role == "AXStaticText";  // Sometimes editable text shows as this

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
            if let Some(name) = cfstring_to_string(name_nsstring) {
                if !name.is_empty() {
                    app_names.push(name);
                }
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

        // First pass: try exact match (case-insensitive)
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
            let name = cfstring_to_string(name_nsstring).unwrap_or_default();

            // Check for exact match (case-insensitive)
            if name.to_lowercase() == app_name.to_lowercase() {
                // Found exact match! Activate it
                // NSApplicationActivateAllWindows = 1 << 0 = 1
                // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
                // Combine both flags: 1 | 2 = 3
                let options: usize = 3;
                let success: bool = msg_send![app, activateWithOptions: options];

                if success {
                    log::info!("Successfully activated application: {} (exact match for '{}')", name, app_name);
                    return Ok(());
                } else {
                    bail!("Failed to activate application: {}", name);
                }
            }
        }

        // Second pass: try contains match (case-insensitive)
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
            let name = cfstring_to_string(name_nsstring).unwrap_or_default();

            // Check if this app name contains our search string (case-insensitive)
            if name.to_lowercase().contains(&app_name.to_lowercase()) {
                // Found partial match! Activate it
                let options: usize = 3;
                let success: bool = msg_send![app, activateWithOptions: options];

                if success {
                    log::info!("Successfully activated application: {} (partial match for '{}')", name, app_name);
                    return Ok(());
                } else {
                    bail!("Failed to activate application: {}", name);
                }
            }
        }

        bail!("No running application found matching '{}'", app_name)
    }
}
