//! Menu clicking functionality via Accessibility API
//!
//! STATUS: EXPERIMENTAL - Work in progress
//!
//! Current issues:
//! - Calling Accessibility API from background threads causes foreign exceptions
//! - Need proper main thread dispatch mechanism
//! - NSAutoreleasePool and Objective-C exception handling

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub title: String,
    pub enabled: bool,
    pub children: Vec<MenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuBar {
    pub menus: Vec<MenuItem>,
}

/// Helper to get PID for an application by name
///
/// Uses two-pass matching (case-insensitive):
/// 1. First pass: exact match
/// 2. Second pass: contains match
unsafe fn get_pid_for_app(app_name: &str) -> Result<i32> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use core_graphics::window::{kCGWindowListOptionAll, CGWindowListCopyWindowInfo};
    use std::ffi::c_void;

    let window_list = CGWindowListCopyWindowInfo(kCGWindowListOptionAll, 0);
    if window_list.is_null() {
        bail!("Failed to get window list");
    }

    let cf_array = core_foundation::array::CFArray::<core_foundation::dictionary::CFDictionary>::wrap_under_create_rule(window_list as *const _);

    // First pass: exact match (case-insensitive)
    for i in 0..cf_array.len() {
        if let Some(window) = cf_array.get(i) {
            let owner_name_key = CFString::from_static_string("kCGWindowOwnerName");
            if let Some(owner_name_value) = window.find(owner_name_key.as_CFTypeRef()) {
                let owner_name = CFString::wrap_under_get_rule(*owner_name_value as *const _);
                let owner_name_str = owner_name.to_string();
                if owner_name_str.to_lowercase() == app_name.to_lowercase() {
                    // Found exact match, get its PID
                    let pid_key = CFString::from_static_string("kCGWindowOwnerPID");
                    if let Some(pid_value) = window.find(pid_key.as_CFTypeRef()) {
                        let cf_number = core_foundation::number::CFNumber::wrap_under_get_rule(*pid_value as *const _);
                        log::info!("Found app '{}' (exact match for '{}'), PID: {}", owner_name_str, app_name, cf_number.to_i32().unwrap());
                        return Ok(cf_number.to_i32().unwrap());
                    }
                }
            }
        }
    }

    // Second pass: contains match (case-insensitive)
    for i in 0..cf_array.len() {
        if let Some(window) = cf_array.get(i) {
            let owner_name_key = CFString::from_static_string("kCGWindowOwnerName");
            if let Some(owner_name_value) = window.find(owner_name_key.as_CFTypeRef()) {
                let owner_name = CFString::wrap_under_get_rule(*owner_name_value as *const _);
                let owner_name_str = owner_name.to_string();
                if owner_name_str.to_lowercase().contains(&app_name.to_lowercase()) {
                    // Found partial match, get its PID
                    let pid_key = CFString::from_static_string("kCGWindowOwnerPID");
                    if let Some(pid_value) = window.find(pid_key.as_CFTypeRef()) {
                        let cf_number = core_foundation::number::CFNumber::wrap_under_get_rule(*pid_value as *const _);
                        log::info!("Found app '{}' (contains match for '{}'), PID: {}", owner_name_str, app_name, cf_number.to_i32().unwrap());
                        return Ok(cf_number.to_i32().unwrap());
                    }
                }
            }
        }
    }

    bail!("Could not find PID for application: {}", app_name)
}

/// Get the menu structure for a specific application
///
/// Returns a clean representation of all menus and menu items
pub fn get_app_menus(app_name: &str) -> Result<MenuBar> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use std::ffi::c_void;

    // Check accessibility permissions first
    if !crate::macos::app_info::has_accessibility_permission() {
        bail!("Accessibility permissions not granted. Enable in System Preferences > Security & Privacy > Accessibility");
    }

    unsafe {
        // Get PID for the specified application
        let pid = get_pid_for_app(app_name)?;
        log::info!("Attempting to get menus for app: {} (PID: {})", app_name, pid);

        // Create accessibility element for the application
        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            bail!("Failed to create AXUIElement for application");
        }

        // Get the menu bar
        let menu_bar_key = CFString::from_static_string("AXMenuBar");
        let mut menu_bar_value: *const c_void = std::ptr::null();

        let result = AXUIElementCopyAttributeValue(
            app_ref,
            menu_bar_key.as_concrete_TypeRef(),
            &mut menu_bar_value,
        );

        if result != 0 {
            CFRelease(app_ref as *const c_void);
            let error_msg = match result {
                -25212 => "No menu bar value (kAXErrorNoValue). The app may not expose its menu bar via Accessibility API.",
                -25205 => "Menu bar attribute not supported (kAXErrorAttributeUnsupported)",
                -25211 => "Accessibility API is disabled (kAXErrorAPIDisabled)",
                -25202 => "Invalid UI element (kAXErrorInvalidUIElement)",
                _ => "Unknown error",
            };
            bail!("Failed to get menu bar for {}: {} (error code: {})", app_name, error_msg, result);
        }

        if menu_bar_value.is_null() {
            CFRelease(app_ref as *const c_void);
            bail!("Menu bar value is null for {}", app_name);
        }

        let menu_bar_ref = menu_bar_value as AXUIElementRef;

        // Get menu bar children (the actual menus)
        let children_key = CFString::from_static_string("AXChildren");
        let mut children_value: *const c_void = std::ptr::null();

        let result = AXUIElementCopyAttributeValue(
            menu_bar_ref,
            children_key.as_concrete_TypeRef(),
            &mut children_value,
        );

        if result != 0 || children_value.is_null() {
            CFRelease(menu_bar_ref as *const c_void);
            CFRelease(app_ref as *const c_void);
            bail!("Failed to get menu bar children");
        }

        let children_array = core_foundation::array::CFArray::<AXUIElementRef>::wrap_under_create_rule(children_value as *const _);

        let mut menus = Vec::new();
        for i in 0..children_array.len() {
            if let Some(menu_ref) = children_array.get(i) {
                if let Ok(menu_item) = get_menu_item_details(*menu_ref) {
                    menus.push(menu_item);
                }
            }
        }

        CFRelease(menu_bar_ref as *const c_void);
        CFRelease(app_ref as *const c_void);

        Ok(MenuBar { menus })
    }
}

/// Helper function to recursively get menu item details
unsafe fn get_menu_item_details(element: AXUIElementRef) -> Result<MenuItem> {
    use core_foundation::base::TCFType;
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;

    // Get title
    let title_key = CFString::from_static_string("AXTitle");
    let mut title_value: *const c_void = std::ptr::null();
    AXUIElementCopyAttributeValue(
        element,
        title_key.as_concrete_TypeRef(),
        &mut title_value,
    );

    let title = if !title_value.is_null() {
        let cf_string = CFString::wrap_under_create_rule(title_value as CFStringRef);
        cf_string.to_string()
    } else {
        String::from("(no title)")
    };

    // Get enabled state
    let enabled_key = CFString::from_static_string("AXEnabled");
    let mut enabled_value: *const c_void = std::ptr::null();
    AXUIElementCopyAttributeValue(
        element,
        enabled_key.as_concrete_TypeRef(),
        &mut enabled_value,
    );

    let enabled = if !enabled_value.is_null() {
        let cf_bool = core_foundation::boolean::CFBoolean::wrap_under_get_rule(enabled_value as *const _);
        cf_bool.into()
    } else {
        true
    };

    // Get children (submenu items)
    let children_key = CFString::from_static_string("AXChildren");
    let mut children_value: *const c_void = std::ptr::null();
    AXUIElementCopyAttributeValue(
        element,
        children_key.as_concrete_TypeRef(),
        &mut children_value,
    );

    let mut children = Vec::new();
    if !children_value.is_null() {
        let children_array = core_foundation::array::CFArray::<AXUIElementRef>::wrap_under_get_rule(children_value as *const _);

        for i in 0..children_array.len() {
            if let Some(child_ref) = children_array.get(i) {
                if let Ok(child_item) = get_menu_item_details(*child_ref) {
                    children.push(child_item);
                }
            }
        }
    }

    Ok(MenuItem {
        title,
        enabled,
        children,
    })
}

/// Click a menu item in an application
///
/// Navigates through menu hierarchy and clicks the final item
///
/// Matching behavior (all case-insensitive):
/// - App name: Exact match first, then contains match
/// - Menu items: Case-insensitive matching
///
/// # Example
/// ```ignore
/// run_menu_item("Pro Tools", &["File", "Save Session As"])?;
/// run_menu_item("pro tools", &["file", "save session as"])?; // All case-insensitive
/// run_menu_item("Pro", &["File", "New Session"])?;           // Partial app name
/// ```
pub fn run_menu_item(app_name: &str, menu_path: &[&str]) -> Result<()> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use std::ffi::c_void;

    if menu_path.is_empty() {
        bail!("Menu path cannot be empty");
    }

    // Check accessibility permissions
    if !crate::macos::app_info::has_accessibility_permission() {
        bail!("Accessibility permissions not granted");
    }

    unsafe {
        let pid = get_pid_for_app(app_name)?;
        log::info!("Clicking menu item in {} (PID: {}): {:?}", app_name, pid, menu_path);

        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            bail!("Failed to create AXUIElement for application");
        }

        // Get the menu bar
        let menu_bar_key = CFString::from_static_string("AXMenuBar");
        let mut menu_bar_value: *const c_void = std::ptr::null();

        let result = AXUIElementCopyAttributeValue(
            app_ref,
            menu_bar_key.as_concrete_TypeRef(),
            &mut menu_bar_value,
        );

        if result != 0 || menu_bar_value.is_null() {
            CFRelease(app_ref as *const c_void);
            bail!("Failed to get menu bar (error: {})", result);
        }

        let menu_bar_ref = menu_bar_value as AXUIElementRef;

        // Navigate through the menu path
        let mut current_element = menu_bar_ref;

        for (i, &menu_title) in menu_path.iter().enumerate() {
            log::info!("Looking for menu item: {}", menu_title);

            // Get children of current element
            let children_key = CFString::from_static_string("AXChildren");
            let mut children_value: *const c_void = std::ptr::null();

            let result = AXUIElementCopyAttributeValue(
                current_element,
                children_key.as_concrete_TypeRef(),
                &mut children_value,
            );

            if result != 0 || children_value.is_null() {
                CFRelease(menu_bar_ref as *const c_void);
                CFRelease(app_ref as *const c_void);
                bail!("Failed to get children for menu level {} ({})", i, menu_title);
            }

            let children_array = core_foundation::array::CFArray::<AXUIElementRef>::wrap_under_get_rule(children_value as *const _);

            // Find the menu item with matching title
            let mut found = false;
            for j in 0..children_array.len() {
                if let Some(child) = children_array.get(j) {
                    let title_key = CFString::from_static_string("AXTitle");
                    let mut title_value: *const c_void = std::ptr::null();

                    AXUIElementCopyAttributeValue(
                        *child,
                        title_key.as_concrete_TypeRef(),
                        &mut title_value,
                    );

                    if !title_value.is_null() {
                        let cf_string = CFString::wrap_under_create_rule(title_value as *const _);
                        let title = cf_string.to_string();

                        // Case-insensitive comparison
                        if title.to_lowercase() == menu_title.to_lowercase() {
                            log::info!("Found menu item: {} (matched '{}')", title, menu_title);

                            // If this is the last item in path, click it
                            if i == menu_path.len() - 1 {
                                log::info!("Clicking final menu item: {}", menu_title);
                                let press_key = CFString::from_static_string("AXPress");
                                let press_result = AXUIElementPerformAction(*child, press_key.as_concrete_TypeRef());

                                CFRelease(menu_bar_ref as *const c_void);
                                CFRelease(app_ref as *const c_void);

                                if press_result == 0 {
                                    log::info!("✅ Successfully clicked menu item");
                                    return Ok(());
                                } else {
                                    bail!("Failed to press menu item (error: {})", press_result);
                                }
                            } else {
                                // For top-level menu items (like "File"), we need to go deeper
                                // Get the actual menu content (usually first child)
                                let mut submenu_value: *const c_void = std::ptr::null();
                                AXUIElementCopyAttributeValue(
                                    *child,
                                    children_key.as_concrete_TypeRef(),
                                    &mut submenu_value,
                                );

                                if !submenu_value.is_null() {
                                    let submenu_array = core_foundation::array::CFArray::<AXUIElementRef>::wrap_under_get_rule(submenu_value as *const _);
                                    if submenu_array.len() > 0 {
                                        if let Some(submenu) = submenu_array.get(0) {
                                            // This is the actual menu container, use it as current element
                                            current_element = *submenu;
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !found && i < menu_path.len() - 1 {
                CFRelease(menu_bar_ref as *const c_void);
                CFRelease(app_ref as *const c_void);
                bail!("Could not find menu item: {} at level {}", menu_title, i);
            }
        }

        CFRelease(menu_bar_ref as *const c_void);
        CFRelease(app_ref as *const c_void);

        bail!("Menu navigation completed but item not clicked")
    }
}

// FFI declarations for Accessibility API
type AXUIElementRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut *const c_void,
    ) -> i32;
    fn AXUIElementPerformAction(
        element: AXUIElementRef,
        action: CFStringRef,
    ) -> i32;
    fn CFRelease(cf: *const c_void);
}

use std::ffi::c_void;
use core_foundation::string::CFStringRef;
