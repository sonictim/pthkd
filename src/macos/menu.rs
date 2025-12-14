//! Menu clicking functionality via Accessibility API
//!
//! STATUS: EXPERIMENTAL - Work in progress
//!
//! Current issues:
//! - Calling Accessibility API from background threads causes foreign exceptions
//! - Need proper main thread dispatch mechanism
//! - NSAutoreleasePool and Objective-C exception handling

use super::ffi::*;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

// Type alias for CoreFoundation compatibility (CFArray needs *const)
type CFAXUIElementRef = *const std::ffi::c_void;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
/// Uses soft_match for flexible name matching (case-insensitive, partial matches)
unsafe fn get_pid_for_app(app_name: &str) -> Result<i32> {
    use objc2::msg_send;

    super::helpers::with_running_app(app_name, |app| {
        let pid: i32 = msg_send![app, processIdentifier];
        log::info!("Found app '{}', PID: {}", app_name, pid);
        Ok(pid)
    })
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
        bail!(
            "Accessibility permissions not granted. Enable in System Preferences > Security & Privacy > Accessibility"
        );
    }

    unsafe {
        // Get PID for the specified application
        let pid = get_pid_for_app(app_name)?;
        log::info!(
            "Attempting to get menus for app: {} (PID: {})",
            app_name,
            pid
        );

        // Create accessibility element for the application
        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            bail!("Failed to create AXUIElement for application");
        }

        // Get the menu bar
        let menu_bar_key = CFString::from_static_string("AXMenuBar");
        let mut menu_bar_value: *mut c_void = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            app_ref,
            menu_bar_key.as_concrete_TypeRef() as *mut c_void,
            &mut menu_bar_value,
        );

        if result != 0 {
            CFRelease(app_ref);
            let error_msg = match result {
                -25212 => {
                    "No menu bar value (kAXErrorNoValue). The app may not expose its menu bar via Accessibility API."
                }
                -25205 => "Menu bar attribute not supported (kAXErrorAttributeUnsupported)",
                -25211 => "Accessibility API is disabled (kAXErrorAPIDisabled)",
                -25202 => "Invalid UI element (kAXErrorInvalidUIElement)",
                _ => "Unknown error",
            };
            bail!(
                "Failed to get menu bar for {}: {} (error code: {})",
                app_name,
                error_msg,
                result
            );
        }

        if menu_bar_value.is_null() {
            CFRelease(app_ref);
            bail!("Menu bar value is null for {}", app_name);
        }

        let menu_bar_ref = menu_bar_value;

        // Get menu bar children (the actual menus)
        let children_key = CFString::from_static_string("AXChildren");
        let mut children_value: *mut c_void = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            menu_bar_ref,
            children_key.as_concrete_TypeRef() as *mut c_void,
            &mut children_value,
        );

        if result != 0 || children_value.is_null() {
            CFRelease(menu_bar_ref);
            CFRelease(app_ref);
            bail!("Failed to get menu bar children");
        }

        let children_array =
            core_foundation::array::CFArray::<CFAXUIElementRef>::wrap_under_create_rule(
                children_value as *const _,
            );

        let mut menus = Vec::new();
        for i in 0..children_array.len() {
            if let Some(menu_ref) = children_array.get(i) {
                if let Ok(menu_item) = get_menu_item_details(*menu_ref as AXUIElementRef) {
                    menus.push(menu_item);
                }
            }
        }

        CFRelease(menu_bar_ref);
        CFRelease(app_ref);

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
    let mut title_value: *mut c_void = std::ptr::null_mut();
    AXUIElementCopyAttributeValue(
        element,
        title_key.as_concrete_TypeRef() as *mut c_void,
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
    let mut enabled_value: *mut c_void = std::ptr::null_mut();
    AXUIElementCopyAttributeValue(
        element,
        enabled_key.as_concrete_TypeRef() as *mut c_void,
        &mut enabled_value,
    );

    let enabled = if !enabled_value.is_null() {
        let cf_bool =
            core_foundation::boolean::CFBoolean::wrap_under_get_rule(enabled_value as *const _);
        cf_bool.into()
    } else {
        true
    };

    // Get children (submenu items)
    let children_key = CFString::from_static_string("AXChildren");
    let mut children_value: *mut c_void = std::ptr::null_mut();
    AXUIElementCopyAttributeValue(
        element,
        children_key.as_concrete_TypeRef() as *mut c_void,
        &mut children_value,
    );

    let mut children = Vec::new();
    if !children_value.is_null() {
        let children_array =
            core_foundation::array::CFArray::<CFAXUIElementRef>::wrap_under_get_rule(
                children_value as *const _,
            );

        for i in 0..children_array.len() {
            if let Some(child_ref) = children_array.get(i) {
                if let Ok(child_item) = get_menu_item_details(*child_ref as AXUIElementRef) {
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
        log::info!(
            "Clicking menu item in {} (PID: {}): {:?}",
            app_name,
            pid,
            menu_path
        );

        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            bail!("Failed to create AXUIElement for application");
        }

        // Get the menu bar
        let menu_bar_key = CFString::from_static_string("AXMenuBar");
        let mut menu_bar_value: *mut c_void = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            app_ref,
            menu_bar_key.as_concrete_TypeRef() as *mut c_void,
            &mut menu_bar_value,
        );

        if result != 0 || menu_bar_value.is_null() {
            CFRelease(app_ref);
            bail!("Failed to get menu bar (error: {})", result);
        }

        let menu_bar_ref = menu_bar_value;

        // Navigate through the menu path
        let mut current_element = menu_bar_ref;

        for (i, &menu_title) in menu_path.iter().enumerate() {
            log::info!("Looking for menu item: {}", menu_title);

            // Get children of current element
            let children_key = CFString::from_static_string("AXChildren");
            let mut children_value: *mut c_void = std::ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                current_element,
                children_key.as_concrete_TypeRef() as *mut c_void,
                &mut children_value,
            );

            if result != 0 || children_value.is_null() {
                CFRelease(menu_bar_ref);
                CFRelease(app_ref);
                bail!(
                    "Failed to get children for menu level {} ({})",
                    i,
                    menu_title
                );
            }

            let children_array =
                core_foundation::array::CFArray::<CFAXUIElementRef>::wrap_under_get_rule(
                    children_value as *const _,
                );

            // Find the menu item with matching title
            let mut found = false;
            for j in 0..children_array.len() {
                if let Some(child) = children_array.get(j) {
                    let child = *child as AXUIElementRef;
                    let title_key = CFString::from_static_string("AXTitle");
                    let mut title_value: *mut c_void = std::ptr::null_mut();

                    AXUIElementCopyAttributeValue(
                        child,
                        title_key.as_concrete_TypeRef() as *mut c_void,
                        &mut title_value,
                    );

                    if !title_value.is_null() {
                        let cf_string = CFString::wrap_under_create_rule(title_value as *const _);
                        let title = cf_string.to_string();

                        // Normalized exact match (case + whitespace insensitive, but NO partial matching)
                        if crate::normalize(&title) == crate::normalize(menu_title) {
                            log::info!("Found menu item: {} (matched '{}')", title, menu_title);

                            // If this is the last item in path, click it
                            if i == menu_path.len() - 1 {
                                log::info!("Clicking final menu item: {}", menu_title);
                                let press_key = CFString::from_static_string("AXPress");
                                let press_result = AXUIElementPerformAction(
                                    child,
                                    press_key.as_concrete_TypeRef() as *mut c_void,
                                );

                                CFRelease(menu_bar_ref);
                                CFRelease(app_ref);

                                if press_result == 0 {
                                    log::info!("✅ Successfully clicked menu item");
                                    return Ok(());
                                } else {
                                    bail!("Failed to press menu item (error: {})", press_result);
                                }
                            } else {
                                // For top-level menu items (like "File"), we need to go deeper
                                // Get the actual menu content (usually first child)
                                let mut submenu_value: *mut c_void = std::ptr::null_mut();
                                AXUIElementCopyAttributeValue(
                                    child,
                                    children_key.as_concrete_TypeRef() as *mut c_void,
                                    &mut submenu_value,
                                );

                                if !submenu_value.is_null() {
                                    let submenu_array = core_foundation::array::CFArray::<
                                        CFAXUIElementRef,
                                    >::wrap_under_get_rule(
                                        submenu_value as *const _
                                    );
                                    if submenu_array.len() > 0 {
                                        if let Some(submenu) = submenu_array.get(0) {
                                            // This is the actual menu container, use it as current element
                                            current_element = *submenu as AXUIElementRef;
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
                CFRelease(menu_bar_ref);
                CFRelease(app_ref);
                bail!("Could not find menu item: {} at level {}", menu_title, i);
            }
        }

        CFRelease(menu_bar_ref);
        CFRelease(app_ref);

        bail!("Menu navigation completed but item not clicked")
    }
}

use std::ffi::c_void;
