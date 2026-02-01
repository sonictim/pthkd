//! Swift UI library bridge

use super::ffi::*;
use anyhow::{Result as R, bail};
use libc::c_void;
use std::ffi::CStr;
use std::os::raw::c_char;

#[link(name = "PTHKDui")]
unsafe extern "C" {
    fn pthkd_get_app_menus(app_name: *const c_char) -> *const c_char;
    fn pthkd_menu_click(
        app_name: *const c_char,
        menu_path: *const *const c_char,
        menu_path_count: i32,
    ) -> bool;
    fn pthkd_menu_item_exists(
        app_name: *const c_char,
        menu_path: *const *const c_char,
        menu_path_count: i32,
    ) -> bool;
    fn pthkd_menu_item_enabled(
        app_name: *const c_char,
        menu_path: *const *const c_char,
        menu_path_count: i32,
    ) -> bool;
    fn pthkd_send_keystroke(
        app_name: *const c_char,
        key_char: *const c_char,
        modifiers: i32,
    ) -> bool;
    fn pthkd_send_global_keystroke(
        key_codes: *const u16,
        key_codes_count: i32,
        modifier_flags: u64,
    ) -> bool;
    fn pthkd_type_text(text: *const c_char, mark_events: bool) -> bool;
    fn pthkd_paste_text(text: *const c_char) -> bool;
    fn pthkd_paste_into_focused_field(text: *const c_char, send_enter: bool) -> bool;
    fn pthkd_click_button(
        app_name: *const c_char,
        window_name: *const c_char,
        button_name: *const c_char,
    ) -> bool;
    fn pthkd_click_checkbox(
        app_name: *const c_char,
        window_name: *const c_char,
        checkbox_name: *const c_char,
    ) -> bool;
    fn pthkd_get_window_buttons(
        app_name: *const c_char,
        window_name: *const c_char,
    ) -> *const c_char;
    fn pthkd_set_checkbox_value(
        app_name: *const c_char,
        window_name: *const c_char,
        checkbox_name: *const c_char,
        value: i32,
    ) -> bool;
    fn pthkd_get_popup_menu_items(
        app_name: *const c_char,
        window_name: *const c_char,
        popup_name: *const c_char,
    ) -> *const c_char;
    fn pthkd_get_window_text(app_name: *const c_char, window_name: *const c_char) -> *const c_char;
    fn pthkd_free_string(ptr: *const c_char);

    // App operations
    fn pthkd_get_frontmost_info() -> *const c_char;
    fn pthkd_get_running_apps() -> *const c_char;
    fn pthkd_focus_app(
        app_name: *const c_char,
        window_name: *const c_char,
        should_switch: bool,
        should_launch: bool,
        timeout: i32,
    ) -> bool;
    fn pthkd_launch_app(app_name: *const c_char) -> bool;
    fn pthkd_is_in_text_field() -> bool;

    // Window operations
    fn pthkd_window_exists(app_name: *const c_char, window_name: *const c_char) -> bool;
    fn pthkd_get_window_titles(app_name: *const c_char) -> *const c_char;
    fn pthkd_wait_for_window(
        app_name: *const c_char,
        window_name: *const c_char,
        condition: i32,
        timeout: i32,
    ) -> bool;
    fn pthkd_close_window(
        app_name: *const c_char,
        window_name: *const c_char,
        retry_timeout: i32,
    ) -> bool;
}

/// Get menu structure for an app
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
pub fn get_app_menus(app_name: &str) -> R<String> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let json_ptr = pthkd_get_app_menus(app_cstr.as_ptr());

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();

        pthkd_free_string(json_ptr);

        Ok(json)
    }
}

/// Click a menu item by traversing the menu path
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `menu_path` - Array of menu titles to traverse (e.g. &["File", "Save"])
pub fn menu_click(app_name: &str, menu_path: &[&str]) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;

        // Convert menu path to array of C strings
        let path_cstrs: Vec<CString> = menu_path
            .iter()
            .map(|s| CString::new(*s))
            .collect::<R<Vec<_>, _>>()?;

        let path_ptrs: Vec<*const c_char> = path_cstrs.iter().map(|cs| cs.as_ptr()).collect();

        let success = pthkd_menu_click(
            app_cstr.as_ptr(),
            path_ptrs.as_ptr(),
            menu_path.len() as i32,
        );

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Menu click failed"))
        }
    }
}

/// Check if a menu item exists
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `menu_path` - Array of menu titles to traverse (e.g. &["File", "Save"])
pub fn menu_item_exists(app_name: &str, menu_path: &[&str]) -> R<bool> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let path_cstrs: Vec<CString> = menu_path
            .iter()
            .map(|s| CString::new(*s))
            .collect::<R<Vec<_>, _>>()?;
        let path_ptrs: Vec<*const c_char> = path_cstrs.iter().map(|cs| cs.as_ptr()).collect();

        Ok(pthkd_menu_item_exists(
            app_cstr.as_ptr(),
            path_ptrs.as_ptr(),
            menu_path.len() as i32,
        ))
    }
}

/// Check if a menu item exists and is enabled
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `menu_path` - Array of menu titles to traverse (e.g. &["File", "Save"])
pub fn menu_item_enabled(app_name: &str, menu_path: &[&str]) -> R<bool> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let path_cstrs: Vec<CString> = menu_path
            .iter()
            .map(|s| CString::new(*s))
            .collect::<R<Vec<_>, _>>()?;
        let path_ptrs: Vec<*const c_char> = path_cstrs.iter().map(|cs| cs.as_ptr()).collect();

        Ok(pthkd_menu_item_enabled(
            app_cstr.as_ptr(),
            path_ptrs.as_ptr(),
            menu_path.len() as i32,
        ))
    }
}

/// Send a keystroke to an application
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `key_char` - Key character to send (e.g. "s", "n", "f1")
/// * `modifiers` - Bit flags: shift=1, control=2, option=4, command=8
pub fn send_keystroke(app_name: &str, key_char: &str, modifiers: i32) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let key_cstr = CString::new(key_char)?;

        let success = pthkd_send_keystroke(app_cstr.as_ptr(), key_cstr.as_ptr(), modifiers);

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Keystroke send failed"))
        }
    }
}

/// Send a global keystroke with multiple keys and modifiers
///
/// # Arguments
/// * `key_codes` - Array of key codes to send
/// * `modifier_flags` - CGEventFlags (shift=0x20000, control=0x40000, option=0x80000, command=0x100000)
pub fn send_global_keystroke(key_codes: &[u16], modifier_flags: u64) -> R<()> {
    unsafe {
        let success =
            pthkd_send_global_keystroke(key_codes.as_ptr(), key_codes.len() as i32, modifier_flags);

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Global keystroke send failed"))
        }
    }
}

/// Type text character by character
///
/// # Arguments
/// * `text` - The text string to type
/// * `mark_events` - Whether to mark events with APP_MARKER (true = prevent event tap from catching them)
pub fn type_text(text: &str, mark_events: bool) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let text_cstr = CString::new(text)?;

        let success = pthkd_type_text(text_cstr.as_ptr(), mark_events);

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Type text failed"))
        }
    }
}

/// Paste text using clipboard and Cmd+V
///
/// This is useful for password fields that may filter out programmatic keystrokes.
/// Works by:
/// 1. Saving current clipboard
/// 2. Setting clipboard to the text
/// 3. Sending Cmd+V
/// 4. Restoring previous clipboard
///
/// # Arguments
/// * `text` - The text to paste
pub fn paste_text(text: &str) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let text_cstr = CString::new(text)?;

        let success = pthkd_paste_text(text_cstr.as_ptr());

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Paste text failed"))
        }
    }
}

/// Paste text into the focused field using Accessibility API
/// If send_enter is true, Swift sends Enter key after pasting (atomic operation)
pub fn paste_into_focused_field(text: &str, send_enter: bool) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let text_cstr = CString::new(text)?;

        let success = pthkd_paste_into_focused_field(text_cstr.as_ptr(), send_enter);

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Paste into focused field failed"))
        }
    }
}

/// Click a button in a window
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
/// * `button_name` - Name of the button to click
pub fn click_button(app_name: &str, window_name: &str, button_name: &str) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;
        let button_cstr = CString::new(button_name)?;

        let success = pthkd_click_button(
            app_cstr.as_ptr(),
            window_cstr.as_ptr(),
            button_cstr.as_ptr(),
        );

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Button click failed"))
        }
    }
}

/// Click a checkbox in a window
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
/// * `checkbox_name` - Name of the checkbox to click
pub fn click_checkbox(app_name: &str, window_name: &str, checkbox_name: &str) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;
        let checkbox_cstr = CString::new(checkbox_name)?;

        let success = pthkd_click_checkbox(
            app_cstr.as_ptr(),
            window_cstr.as_ptr(),
            checkbox_cstr.as_ptr(),
        );

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Checkbox click failed"))
        }
    }
}

/// Get list of buttons in a window
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
///   Helper to check if JSON response contains an error
fn check_swift_error(json: &str) -> R<()> {
    #[derive(serde::Deserialize)]
    struct ErrorResponse {
        error: String,
    }

    if let Ok(err_resp) = serde_json::from_str::<ErrorResponse>(json) {
        return Err(anyhow::anyhow!("Swift error: {}", err_resp.error));
    }
    Ok(())
}

pub fn get_window_buttons(app_name: &str, window_name: &str) -> R<Vec<String>> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;

        let json_ptr = pthkd_get_window_buttons(app_cstr.as_ptr(), window_cstr.as_ptr());

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        // Check for error response first
        check_swift_error(&json)?;

        let buttons: Vec<String> = serde_json::from_str(&json)?;
        Ok(buttons)
    }
}

/// Set checkbox to specific value (checked/unchecked)
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
/// * `checkbox_name` - Name of the checkbox
/// * `value` - 0 for unchecked, 1 for checked
pub fn set_checkbox_value(
    app_name: &str,
    window_name: &str,
    checkbox_name: &str,
    value: i32,
) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;
        let checkbox_cstr = CString::new(checkbox_name)?;

        let success = pthkd_set_checkbox_value(
            app_cstr.as_ptr(),
            window_cstr.as_ptr(),
            checkbox_cstr.as_ptr(),
            value,
        );

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Set checkbox value failed"))
        }
    }
}

/// Get items from a popup menu
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
/// * `popup_name` - Name of the popup button
pub fn get_popup_menu_items(app_name: &str, window_name: &str, popup_name: &str) -> R<Vec<String>> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;
        let popup_cstr = CString::new(popup_name)?;

        let json_ptr = pthkd_get_popup_menu_items(
            app_cstr.as_ptr(),
            window_cstr.as_ptr(),
            popup_cstr.as_ptr(),
        );

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        // Check for error response first
        check_swift_error(&json)?;

        let items: Vec<String> = serde_json::from_str(&json)?;
        Ok(items)
    }
}

/// Get all text from a window
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
pub fn get_window_text(app_name: &str, window_name: &str) -> R<Vec<String>> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;

        let json_ptr = pthkd_get_window_text(app_cstr.as_ptr(), window_cstr.as_ptr());

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        // Check for error response first
        check_swift_error(&json)?;

        let text: Vec<String> = serde_json::from_str(&json)?;
        Ok(text)
    }
}

// MARK: - App Operations

/// Information about the frontmost application and window
#[derive(Debug, serde::Deserialize)]
pub struct FrontmostInfo {
    pub app: String,
    pub window: String,
}

/// Get information about the frontmost application and window
pub fn get_frontmost_info() -> R<FrontmostInfo> {
    unsafe {
        let json_ptr = pthkd_get_frontmost_info();

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        // Check for error response first
        check_swift_error(&json)?;

        let info: FrontmostInfo = serde_json::from_str(&json)?;
        Ok(info)
    }
}

/// Get list of all running application names
pub fn get_running_apps() -> R<Vec<String>> {
    unsafe {
        let json_ptr = pthkd_get_running_apps();

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        // Check for error response first
        check_swift_error(&json)?;

        let apps: Vec<String> = serde_json::from_str(&json)?;
        Ok(apps)
    }
}

/// Focus/activate an application
///
/// # Arguments
/// * `app_name` - Name of app to focus (empty string = no change)
/// * `window_name` - Name of specific window to wait for (empty = any window)
/// * `should_switch` - Whether to switch to the app
/// * `should_launch` - Whether to launch if not running
/// * `timeout` - Maximum time to wait in milliseconds
pub fn focus_app(
    app_name: &str,
    window_name: &str,
    should_switch: bool,
    should_launch: bool,
    timeout: i32,
) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;

        let success = pthkd_focus_app(
            app_cstr.as_ptr(),
            window_cstr.as_ptr(),
            should_switch,
            should_launch,
            timeout,
        );

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Focus app failed"))
        }
    }
}

/// Launch an application
///
/// # Arguments
/// * `app_name` - Name of the application to launch
pub fn launch_app(app_name: &str) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let success = pthkd_launch_app(app_cstr.as_ptr());

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Launch app failed"))
        }
    }
}

/// Check if the currently focused UI element is a text input field
///
/// Returns true if focused element is a text field, text area, combo box, or search field.
/// This is useful for preventing hotkeys from triggering when the user is typing.
pub fn is_in_text_field() -> bool {
    unsafe { pthkd_is_in_text_field() }
}

// MARK: - Window Operations

/// Check if a window exists
///
/// # Arguments
/// * `app_name` - Name of the app (empty for frontmost)
/// * `window_name` - Name of the window (empty for frontmost)
pub fn window_exists(app_name: &str, window_name: &str) -> R<bool> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;

        Ok(pthkd_window_exists(app_cstr.as_ptr(), window_cstr.as_ptr()))
    }
}

/// Get all window titles for an application
///
/// # Arguments
/// * `app_name` - Name of the app (empty for frontmost)
pub fn get_window_titles(app_name: &str) -> R<Vec<String>> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let json_ptr = pthkd_get_window_titles(app_cstr.as_ptr());

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        // Check for error response first
        check_swift_error(&json)?;

        let titles: Vec<String> = serde_json::from_str(&json)?;
        Ok(titles)
    }
}

/// Window condition to wait for
pub enum WindowCondition {
    Exists = 0,
    Closed = 1,
    Focused = 2,
}

/// Wait for a window to meet a specific condition
///
/// # Arguments
/// * `app_name` - Name of the app (empty for frontmost)
/// * `window_name` - Name of the window (empty for frontmost)
/// * `condition` - Condition to wait for
/// * `timeout` - Maximum time to wait in milliseconds
///
/// Returns true if condition was met, false if timeout
pub fn wait_for_window(
    app_name: &str,
    window_name: &str,
    condition: WindowCondition,
    timeout: i32,
) -> R<bool> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;

        Ok(pthkd_wait_for_window(
            app_cstr.as_ptr(),
            window_cstr.as_ptr(),
            condition as i32,
            timeout,
        ))
    }
}

/// Close a window
///
/// # Arguments
/// * `app_name` - Name of the app (empty for frontmost)
/// * `window_name` - Name of the window (empty for frontmost)
/// * `retry_timeout` - If Some, retry closing until window is gone or timeout (in milliseconds)
pub fn close_window(app_name: &str, window_name: &str, retry_timeout: Option<i32>) -> R<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let window_cstr = CString::new(window_name)?;
        let retry = retry_timeout.unwrap_or(-1);

        let success = pthkd_close_window(app_cstr.as_ptr(), window_cstr.as_ptr(), retry);

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Close window failed"))
        }
    }
}

/// Get the name of the currently focused (frontmost) application
///
/// # Example
/// ```ignore
/// let app_name = get_current_app()?;
/// println!("Current app: {}", app_name); // "Pro Tools"
pub fn get_current_app() -> R<String> {
    let info = get_frontmost_info()?;
    Ok(info.app)
}

/// Get the title of the currently focused window
///
/// # Example
/// ```ignore
/// let window_title = get_app_window()?;
/// println!("Window: {}", window_title); // "My Session - Pro Tools"
/// ```
pub fn get_app_window() -> R<String> {
    let info = get_frontmost_info()?;
    Ok(info.window)
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
pub fn get_pid_by_name(app_name: &str) -> R<i32> {
    use objc2::msg_send;

    unsafe {
        super::helpers::with_running_app(app_name, |app| {
            let pid: i32 = msg_send![app, processIdentifier];
            Ok(pid)
        })
    }
}

// ============================================================================
// Core Graphics Event FFI
// ============================================================================

const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
const CG_HID_EVENT_TAP: u32 = 0;
const CG_SESSION_EVENT_TAP: u32 = 1;

// Modifier key flags
const CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
const CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
const CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000; // Option key
const CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;

unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;

    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CGEventSourceCreate(source_state_id: i32) -> *mut c_void;
    fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
}

// ============================================================================
// Modifier Helpers
// ============================================================================

/// Check if a key name is a modifier
fn is_modifier(key_name: &str) -> bool {
    matches!(
        key_name.to_lowercase().as_str(),
        "cmd" | "command" | "shift" | "option" | "alt" | "control" | "ctrl"
    )
}

/// Convert modifier key name to flag
fn modifier_to_flag(key_name: &str) -> Option<u64> {
    match key_name.to_lowercase().as_str() {
        "cmd" | "command" => Some(CG_EVENT_FLAG_MASK_COMMAND),
        "shift" => Some(CG_EVENT_FLAG_MASK_SHIFT),
        "option" | "alt" => Some(CG_EVENT_FLAG_MASK_ALTERNATE),
        "control" | "ctrl" => Some(CG_EVENT_FLAG_MASK_CONTROL),
        _ => None,
    }
}

// ============================================================================
// Global Keystroke Sending
// ============================================================================

/// Send a global keystroke chord
///
/// **STATUS: WORKING** - Uses pure C Core Graphics API
///
/// This posts keyboard events to the system event queue. The keystrokes
/// go to whatever application currently has focus.
///
/// Modifiers (cmd, shift, option, control) are applied as flags on the
/// regular key events, not sent as separate key events. This is the proper
/// way macOS applications expect to receive modified keystrokes.
///
/// # Arguments
/// * `keys` - Slice of key names to press simultaneously (e.g., &["cmd", "f1"])
///
/// # Example
/// ```ignore
/// // Send Cmd+S
/// send_keystroke(&["cmd", "s"])?;
///
/// // Send just Space
/// send_keystroke(&["space"])?;
///
/// // Send Cmd+Shift+F1
/// send_keystroke(&["cmd", "shift", "f1"])?;
/// ```
pub fn keystroke(keys: &[&str]) -> R<()> {
    use crate::input::key_name_to_codes;

    if keys.is_empty() {
        bail!("No keys specified");
    }

    log::debug!("send_keystroke called with keys: {:?}", keys);

    // Separate modifiers from regular keys
    let mut modifier_flags = 0u64;
    let mut regular_keys = Vec::new();

    for &key_name in keys {
        if is_modifier(key_name) {
            // It's a modifier - add to flags
            if let Some(flag) = modifier_to_flag(key_name) {
                modifier_flags |= flag;
                log::debug!("  Added modifier: {} (flag: 0x{:x})", key_name, flag);
            }
        } else {
            // It's a regular key - add to list
            regular_keys.push(key_name);
        }
    }

    // Must have at least one regular key
    if regular_keys.is_empty() {
        bail!("Must specify at least one non-modifier key");
    }

    // Parse regular key names to keycodes
    let mut key_codes = Vec::new();
    for key_name in regular_keys {
        let codes = key_name_to_codes(key_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown key name: {}", key_name))?;

        // For keys with multiple options, use the first one
        let keycode = codes[0];
        key_codes.push(keycode);
        log::debug!("  Key '{}' -> keycode {}", key_name, keycode);
    }

    // Use Swift bridge to send keystroke
    send_global_keystroke(&key_codes, modifier_flags)
}
