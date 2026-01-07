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
