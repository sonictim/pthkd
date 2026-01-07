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
