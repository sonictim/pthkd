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
    unsafe {
        use objc2::msg_send;

        super::helpers::with_running_app(app_name, |app| {
            let pid: i32 = msg_send![app, processIdentifier];
            log::info!("Found app '{}', PID: {}", app_name, pid);
            Ok(pid)
        })
    }
}

/// Get the menu structure for a specific application
///
/// Returns a clean representation of all menus and menu items
pub fn get_app_menus(app_name: &str) -> Result<MenuBar> {
    use super::helpers::CFArray;
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
        let menu_bar_key = create_cfstring("AXMenuBar");
        let mut menu_bar_value: *mut c_void = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            app_ref,
            menu_bar_key,
            &mut menu_bar_value,
        );
        CFRelease(menu_bar_key);

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
        let children_key = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            menu_bar_ref,
            children_key,
            &mut children_value,
        );
        CFRelease(children_key);

        if result != 0 || children_value.is_null() {
            CFRelease(menu_bar_ref);
            CFRelease(app_ref);
            bail!("Failed to get menu bar children");
        }

        let children_array = CFArray::new(children_value);

        let mut menus = Vec::new();
        for i in 0..children_array.count() {
            let menu_ref = children_array.get(i);
            if let Ok(menu_item) = get_menu_item_details(menu_ref as AXUIElementRef) {
                menus.push(menu_item);
            }
        }

        CFRelease(menu_bar_ref);
        CFRelease(app_ref);

        Ok(MenuBar { menus })
    }
}

/// Helper function to recursively get menu item details
unsafe fn get_menu_item_details(element: AXUIElementRef) -> Result<MenuItem> {
    unsafe {
        use super::helpers::CFArray;
        use std::ffi::c_void;

        // Get title
        let title_key = create_cfstring("AXTitle");
        let mut title_value: *mut c_void = std::ptr::null_mut();
        AXUIElementCopyAttributeValue(
            element,
            title_key,
            &mut title_value,
        );
        CFRelease(title_key);

        let title = if !title_value.is_null() {
            cfstring_to_string(title_value).unwrap_or_else(|| String::from("(no title)"))
        } else {
            String::from("(no title)")
        };

        // Get enabled state
        let enabled_key = create_cfstring("AXEnabled");
        let mut enabled_value: *mut c_void = std::ptr::null_mut();
        AXUIElementCopyAttributeValue(
            element,
            enabled_key,
            &mut enabled_value,
        );
        CFRelease(enabled_key);

        let enabled = if !enabled_value.is_null() {
            let result = enabled_value == kCFBooleanTrue;
            CFRelease(enabled_value);
            result
        } else {
            true
        };

        // Get children (submenu items)
        let children_key = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = std::ptr::null_mut();
        AXUIElementCopyAttributeValue(
            element,
            children_key,
            &mut children_value,
        );
        CFRelease(children_key);

        let mut children = Vec::new();
        if !children_value.is_null() {
            let children_array = CFArray::new(children_value);

            for i in 0..children_array.count() {
                let child_ref = children_array.get(i);
                if let Ok(child_item) = get_menu_item_details(child_ref as AXUIElementRef) {
                    children.push(child_item);
                }
            }
        }

        Ok(MenuItem {
            title,
            enabled,
            children,
        })
    }
}

// ============================================================================
// Menu Item Navigation and Actions
// ============================================================================

/// Internal helper to navigate to a menu item by path
///
/// Returns the AXUIElement for the target menu item along with refs that need cleanup.
/// Caller is responsible for calling CFRelease on all returned refs.
///
/// # Safety
/// Unsafe because it uses raw Core Foundation pointers. Caller must CFRelease all returned refs.
unsafe fn navigate_to_menu_item(
    app_name: &str,
    menu_path: &[&str],
) -> Result<(AXUIElementRef, AXUIElementRef, AXUIElementRef)> {
    unsafe {
        use super::helpers::CFArray;
        use std::ffi::c_void;

        if menu_path.is_empty() {
            bail!("Menu path cannot be empty");
        }

        if !crate::macos::app_info::has_accessibility_permission() {
            bail!("Accessibility permissions not granted");
        }

        let pid = get_pid_for_app(app_name)?;
        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            bail!("Failed to create AXUIElement for application");
        }

        // Get the menu bar
        let menu_bar_key = create_cfstring("AXMenuBar");
        let mut menu_bar_value: *mut c_void = std::ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(
            app_ref,
            menu_bar_key,
            &mut menu_bar_value,
        );
        CFRelease(menu_bar_key);

        if result != 0 || menu_bar_value.is_null() {
            CFRelease(app_ref);
            bail!("Failed to get menu bar (error: {})", result);
        }

        let menu_bar_ref = menu_bar_value;
        let mut current_element = menu_bar_ref;

        // Navigate through the menu path
        for (i, &menu_title) in menu_path.iter().enumerate() {
            let children_key = create_cfstring("AXChildren");
            let mut children_value: *mut c_void = std::ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                current_element,
                children_key,
                &mut children_value,
            );
            CFRelease(children_key);

            if result != 0 || children_value.is_null() {
                CFRelease(menu_bar_ref);
                CFRelease(app_ref);
                bail!(
                    "Failed to get children for menu level {} ({})",
                    i,
                    menu_title
                );
            }

            let children_array = CFArray::new(children_value);

            // Find the menu item with matching title
            let mut found_element: Option<AXUIElementRef> = None;
            for j in 0..children_array.count() {
                let child = children_array.get(j) as AXUIElementRef;
                let title_key = create_cfstring("AXTitle");
                let mut title_value: *mut c_void = std::ptr::null_mut();

                AXUIElementCopyAttributeValue(
                    child,
                    title_key,
                    &mut title_value,
                );
                CFRelease(title_key);

                if !title_value.is_null() {
                    let title = cfstring_to_string(title_value).unwrap_or_default();

                    if crate::normalize(&title) == crate::normalize(menu_title) {
                        // Found the item!
                        if i == menu_path.len() - 1 {
                            // This is the final item - return it
                            return Ok((child, menu_bar_ref, app_ref));
                        } else {
                            // Navigate deeper into submenu
                            let children_key2 = create_cfstring("AXChildren");
                            let mut submenu_value: *mut c_void = std::ptr::null_mut();
                            AXUIElementCopyAttributeValue(
                                child,
                                children_key2,
                                &mut submenu_value,
                            );
                            CFRelease(children_key2);

                            if !submenu_value.is_null() {
                                let submenu_array = CFArray::new(submenu_value);
                                if submenu_array.count() > 0 {
                                    let submenu = submenu_array.get(0);
                                    current_element = submenu as AXUIElementRef;
                                    found_element = Some(current_element);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if found_element.is_none() {
                CFRelease(menu_bar_ref);
                CFRelease(app_ref);
                bail!("Could not find menu item: {} at level {}", menu_title, i);
            }
        }

        CFRelease(menu_bar_ref);
        CFRelease(app_ref);
        bail!("Menu navigation completed but target not found")
    }
}

/// Check if a menu item exists
///
/// Returns true if the menu item exists, false otherwise.
/// Does not check if the item is enabled.
///
/// # Example
/// ```ignore
/// if menu_item_exists("Pro Tools", &["File", "Save"]) {
///     println!("Save menu exists!");
/// }
/// ```
pub fn menu_item_exists(app_name: &str, menu_path: &[&str]) -> bool {
    unsafe {
        match navigate_to_menu_item(app_name, menu_path) {
            Ok((_, menu_bar_ref, app_ref)) => {
                CFRelease(menu_bar_ref);
                CFRelease(app_ref);
                true
            }
            Err(_) => false,
        }
    }
}

/// Check if a menu item exists and is enabled
///
/// Returns true if the menu item exists AND is enabled, false otherwise.
///
/// # Example
/// ```ignore
/// if menu_item_enabled("Pro Tools", &["Edit", "Undo"]) {
///     println!("Undo is available!");
/// }
/// ```
pub fn menu_item_enabled(app_name: &str, menu_path: &[&str]) -> bool {
    use std::ffi::c_void;

    unsafe {
        match navigate_to_menu_item(app_name, menu_path) {
            Ok((item_ref, menu_bar_ref, app_ref)) => {
                // Check AXEnabled attribute
                let enabled_key = create_cfstring("AXEnabled");
                let mut enabled_value: *mut c_void = std::ptr::null_mut();

                let result = AXUIElementCopyAttributeValue(
                    item_ref,
                    enabled_key,
                    &mut enabled_value,
                );
                CFRelease(enabled_key);

                CFRelease(menu_bar_ref);
                CFRelease(app_ref);

                if result == 0 && !enabled_value.is_null() {
                    let result = enabled_value == kCFBooleanTrue;
                    CFRelease(enabled_value);
                    result
                } else {
                    // If we can't get enabled status, assume it's disabled
                    false
                }
            }
            Err(_) => false,
        }
    }
}

/// Click a menu item by path
///
/// Navigates through menu hierarchy and clicks the final item.
/// Supports case-insensitive matching for both app names and menu items.
///
/// # Example
/// ```ignore
/// menu_item_run("Pro Tools", &["File", "Save Session As"])?;
/// menu_item_run("pro tools", &["file", "save"])?; // Case-insensitive
/// ```
pub fn menu_item_run(app_name: &str, menu_path: &[&str]) -> Result<()> {
    log::info!("Clicking menu item in {}: {:?}", app_name, menu_path);

    unsafe {
        let (item_ref, menu_bar_ref, app_ref) = navigate_to_menu_item(app_name, menu_path)?;

        // Click the item
        let press_key = create_cfstring("AXPress");
        let press_result =
            AXUIElementPerformAction(item_ref, press_key);
        CFRelease(press_key);

        // Note: item_ref is a borrowed reference from CFArray::get() in navigate_to_menu_item,
        // so we should NOT CFRelease it. Only release the refs we created.
        CFRelease(menu_bar_ref);
        CFRelease(app_ref);

        if press_result == 0 {
            log::info!("✅ Successfully clicked menu item");
            Ok(())
        } else {
            bail!("Failed to press menu item (error: {})", press_result);
        }
    }
}

/// Deprecated: Use menu_item_run() instead
#[deprecated(since = "0.1.0", note = "Use menu_item_run() instead")]
pub fn run_menu_item(app_name: &str, menu_path: &[&str]) -> Result<()> {
    menu_item_run(app_name, menu_path)
}
