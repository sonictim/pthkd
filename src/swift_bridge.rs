//! Swift UI library bridge

use anyhow::Result;
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
pub fn get_app_menus(app_name: &str) -> Result<String> {
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
pub fn menu_click(app_name: &str, menu_path: &[&str]) -> Result<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;

        // Convert menu path to array of C strings
        let path_cstrs: Vec<CString> = menu_path
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<Vec<_>, _>>()?;

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
pub fn menu_item_exists(app_name: &str, menu_path: &[&str]) -> Result<bool> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let path_cstrs: Vec<CString> = menu_path
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<Vec<_>, _>>()?;
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
pub fn menu_item_enabled(app_name: &str, menu_path: &[&str]) -> Result<bool> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let path_cstrs: Vec<CString> = menu_path
            .iter()
            .map(|s| CString::new(*s))
            .collect::<Result<Vec<_>, _>>()?;
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
pub fn send_keystroke(app_name: &str, key_char: &str, modifiers: i32) -> Result<()> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let key_cstr = CString::new(key_char)?;

        let success = pthkd_send_keystroke(
            app_cstr.as_ptr(),
            key_cstr.as_ptr(),
            modifiers,
        );

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Keystroke send failed"))
        }
    }
}

/// Click a button in a window
///
/// # Arguments
/// * `app_name` - Name of the app (e.g. "Pro Tools"), or empty string for frontmost app
/// * `window_name` - Name of the window, or empty string for frontmost window
/// * `button_name` - Name of the button to click
pub fn click_button(app_name: &str, window_name: &str, button_name: &str) -> Result<()> {
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
pub fn click_checkbox(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
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
pub fn get_window_buttons(app_name: &str, window_name: &str) -> Result<Vec<String>> {
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

        let buttons: Vec<String> = serde_json::from_str(&json)?;
        Ok(buttons)
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
pub fn get_frontmost_info() -> Result<FrontmostInfo> {
    unsafe {
        let json_ptr = pthkd_get_frontmost_info();

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

        let info: FrontmostInfo = serde_json::from_str(&json)?;
        Ok(info)
    }
}

/// Get list of all running application names
pub fn get_running_apps() -> Result<Vec<String>> {
    unsafe {
        let json_ptr = pthkd_get_running_apps();

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

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
) -> Result<()> {
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
pub fn launch_app(app_name: &str) -> Result<()> {
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
pub fn window_exists(app_name: &str, window_name: &str) -> Result<bool> {
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
pub fn get_window_titles(app_name: &str) -> Result<Vec<String>> {
    unsafe {
        use std::ffi::CString;

        let app_cstr = CString::new(app_name)?;
        let json_ptr = pthkd_get_window_titles(app_cstr.as_ptr());

        if json_ptr.is_null() {
            return Err(anyhow::anyhow!("Swift returned null"));
        }

        let json = CStr::from_ptr(json_ptr).to_string_lossy().into_owned();
        pthkd_free_string(json_ptr);

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
) -> Result<bool> {
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
pub fn close_window(app_name: &str, window_name: &str, retry_timeout: Option<i32>) -> Result<()> {
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
