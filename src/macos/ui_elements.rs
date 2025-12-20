//! UI element automation for macOS
//!
//! Provides functions to find and interact with UI elements (buttons, etc.)
//! in application windows using the macOS Accessibility API.

use super::app_info::get_pid_by_name;
use super::ffi::*;
use anyhow::{Context, Result, bail};
use libc::c_void;
use std::ptr;

/// Get all buttons in a window
///
/// # Arguments
/// * `app_name` - Name of the application (e.g., "Pro Tools")
/// * `window_name` - Name of the window to search in, or "" for focused window
///
/// # Returns
/// Vector of button titles found in the window
///
/// # Example
/// ```ignore
/// let buttons = get_window_buttons("Pro Tools", "AudioSuite: Reverb")?;
/// // Returns: ["Preview", "Render", "Cancel"]
/// ```
pub fn get_window_buttons(app_name: &str, window_name: &str) -> Result<Vec<String>> {
    unsafe {
        let pid = get_pid_by_name(app_name).context(format!("Failed to find app: {}", app_name))?;

        let app_element = AXUIElementCreateApplication(pid);

        // Get the target window (focused or by name)
        let window = if window_name.is_empty() {
            get_focused_window(app_element)?
        } else {
            find_window_by_name(app_element, window_name)?
        };

        // Get all buttons in the window
        let buttons = find_buttons_in_element(window)?;

        // Clean up
        CFRelease(app_element);
        CFRelease(window);

        Ok(buttons)
    }
}

/// Click a button in a window
///
/// # Arguments
/// * `app_name` - Name of the application (e.g., "Pro Tools")
/// * `window_name` - Name of the window, or "" for focused window
/// * `button_name` - Name of the button to click (soft matched)
///
/// # Example
/// ```ignore
/// // Click "Render" button in the focused window
/// click_button("Pro Tools", "", "Render")?;
///
/// // Click "OK" in a specific plugin window
/// click_button("Pro Tools", "AudioSuite: Reverb", "OK")?;
/// ```
pub fn click_button(app_name: &str, window_name: &str, button_name: &str) -> Result<()> {
    unsafe {
        super::helpers::with_app_window(app_name, window_name, |_app, window| {
            // Find the button
            let button_element = find_button_in_window(window, button_name)?;

            // Click it
            let press_action = create_cfstring("AXPress");
            let result = AXUIElementPerformAction(button_element, press_action);
            CFRelease(press_action);

            if result != K_AX_ERROR_SUCCESS {
                CFRelease(button_element);
                bail!(
                    "Failed to press button '{}' (error code: {})",
                    button_name,
                    result
                );
            }

            CFRelease(button_element);

            log::info!(
                "✅ Clicked button '{}' in window '{}'",
                button_name,
                if window_name.is_empty() {
                    "<focused>"
                } else {
                    window_name
                }
            );

            Ok(())
        })
    }
}
pub fn click_checkbox(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    unsafe {
        super::helpers::with_app_window(app_name, window_name, |_app, window| {
            // Find the checkbox
            let checkbox_element = find_checkbox_in_window(window, checkbox_name)?;

            println!("Found checkbox '{}', attempting to click...", checkbox_name);

            // Toggle it (same AXPress action as buttons)
            let press_action = create_cfstring("AXPress");
            let result = AXUIElementPerformAction(checkbox_element, press_action);
            CFRelease(press_action);

            if result != K_AX_ERROR_SUCCESS {
                CFRelease(checkbox_element);
                bail!(
                    "Failed to toggle checkbox '{}' (error code: {})",
                    checkbox_name,
                    result
                );
            }

            CFRelease(checkbox_element);
            println!("✅ Clicked checkbox '{}'", checkbox_name);
            Ok(())
        })
    }
}

/// Check a checkbox (set to checked state)
pub fn check_box(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    unsafe {
        super::helpers::with_app_window(app_name, window_name, |_app, window| {
            let checkbox_element = find_checkbox_in_window(window, checkbox_name)?;

            println!("Found checkbox '{}', setting to CHECKED...", checkbox_name);

            // Create a CFNumber for value 1 (checked)
            let num_value: i32 = 1;
            let cf_number = CFNumberCreate(
                std::ptr::null(),
                9, // kCFNumberSInt32Type
                &num_value as *const i32 as *const c_void,
            );

            // Set the value
            let value_attr = create_cfstring("AXValue");
            let result = AXUIElementSetAttributeValue(checkbox_element, value_attr, cf_number);
            CFRelease(value_attr);
            CFRelease(cf_number);

            if result != K_AX_ERROR_SUCCESS {
                CFRelease(checkbox_element);
                bail!(
                    "Failed to check checkbox '{}' (error code: {})",
                    checkbox_name,
                    result
                );
            }

            CFRelease(checkbox_element);
            println!("✅ Set checkbox '{}' to CHECKED", checkbox_name);
            Ok(())
        })
    }
}

/// Get menu items from a popup button
pub fn get_popup_menu_items(
    app_name: &str,
    window_name: &str,
    popup_name: &str,
) -> Result<Vec<String>> {
    unsafe {
        super::helpers::with_app_window(app_name, window_name, |_app, window| {
            // Find the popup button
            let popup_element = find_popup_in_window(window, popup_name)?;

            println!("Found popup '{}', opening menu...", popup_name);

            // Click it to open the menu
            let press_action = create_cfstring("AXPress");
            let result = AXUIElementPerformAction(popup_element, press_action);
            CFRelease(press_action);

            // Note: Some apps (like Pro Tools) return error codes even though the popup opens
            // For example, Pro Tools returns -25204 (K_AX_ERROR_INVALID_UI_ELEMENT) but the popup still opens
            if result != K_AX_ERROR_SUCCESS {
                println!(
                    "  ⚠️  AXPress returned error code {} (but popup may still open)",
                    result
                );
            } else {
                println!("  ✅ AXPress succeeded");
            }

            println!("  Waiting for menu to appear...");

            // Wait for menu to appear
            std::thread::sleep(std::time::Duration::from_millis(200));

            println!("  Wait complete, enumerating menu...");

            let mut menu_items = Vec::new();

            // Try method 1: Check for AXMenu attribute on the popup itself
            println!("  Checking for AXMenu attribute...");
            let menu_attr = create_cfstring("AXMenu");
            let mut menu_value: *mut c_void = ptr::null_mut();
            let result = AXUIElementCopyAttributeValue(popup_element, menu_attr, &mut menu_value);
            CFRelease(menu_attr);

            if result == K_AX_ERROR_SUCCESS && !menu_value.is_null() {
                println!("  Found menu via AXMenu attribute!");
                menu_items = get_menu_items_from_menu(menu_value)?;
                CFRelease(menu_value);
            } else {
                println!("  No AXMenu attribute, checking children...");

                // Try method 2: Look in children
                let children_attr = create_cfstring("AXChildren");
                let mut children_value: *mut c_void = ptr::null_mut();
                let result = AXUIElementCopyAttributeValue(
                    popup_element,
                    children_attr,
                    &mut children_value,
                );
                CFRelease(children_attr);

                if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
                    let children_count = CFArrayGetCount(children_value);
                    println!("  Found {} children", children_count);

                    for i in 0..children_count {
                        let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;

                        // Get the role of this child
                        let role_attr = create_cfstring("AXRole");
                        let mut role_value: *mut c_void = ptr::null_mut();
                        AXUIElementCopyAttributeValue(child, role_attr, &mut role_value);
                        CFRelease(role_attr);

                        if let Some(role) = cfstring_to_string(role_value) {
                            if !role_value.is_null() {
                                CFRelease(role_value);
                            }

                            println!("    Child {}: role = {}", i, role);

                            // Look for menu items
                            if role == "AXMenu" {
                                println!("  Found AXMenu child!");
                                menu_items = get_menu_items_from_menu(child)?;
                                break;
                            }
                        }
                    }

                    CFRelease(children_value);
                } else {
                    println!("  No children found");
                }
            }

            CFRelease(popup_element);

            println!("Found {} menu items", menu_items.len());
            for (i, item) in menu_items.iter().enumerate() {
                println!("  {}. {}", i + 1, item);
            }

            Ok(menu_items)
        })
    }
}

/// Get menu item titles from a menu element
unsafe fn get_menu_items_from_menu(menu: AXUIElementRef) -> Result<Vec<String>> {
    unsafe {
        let mut items = Vec::new();

        let children_attr = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(menu, children_attr, &mut children_value);
        CFRelease(children_attr);

        if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
            let children_count = CFArrayGetCount(children_value);

            for i in 0..children_count {
                let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;

                // Get the title of this menu item
                let title_attr = create_cfstring("AXTitle");
                let mut title_value: *mut c_void = ptr::null_mut();
                AXUIElementCopyAttributeValue(child, title_attr, &mut title_value);
                CFRelease(title_attr);

                if let Some(title) = cfstring_to_string(title_value) {
                    if !title_value.is_null() {
                        CFRelease(title_value);
                    }
                    if !title.is_empty() {
                        items.push(title);
                    }
                }
            }

            CFRelease(children_value);
        }

        Ok(items)
    }
}

/// Find a popup button in a window
unsafe fn find_popup_in_window(window: AXUIElementRef, popup_name: &str) -> Result<AXUIElementRef> {
    unsafe {
        find_popup_in_element(window, popup_name)
            .with_context(|| format!("Popup '{}' not found in window", popup_name))
    }
}

/// Recursively find a popup button by name
unsafe fn find_popup_in_element(
    element: AXUIElementRef,
    popup_name: &str,
) -> Result<AXUIElementRef> {
    unsafe {
        // Check if this element is a popup button with matching name
        let role_attr = create_cfstring("AXRole");
        let mut role_value: *mut c_void = ptr::null_mut();
        AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
        CFRelease(role_attr);

        if let Some(role) = cfstring_to_string(role_value) {
            if !role_value.is_null() {
                CFRelease(role_value);
            }

            if role == "AXPopUpButton" {
                let title_attr = create_cfstring("AXTitle");
                let mut title_value: *mut c_void = ptr::null_mut();
                AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
                CFRelease(title_attr);

                if let Some(title) = cfstring_to_string(title_value) {
                    if !title_value.is_null() {
                        CFRelease(title_value);
                    }

                    if crate::soft_match(&title, popup_name) {
                        // Found it! Retain before returning so caller owns it
                        CFRetain(element);
                        return Ok(element);
                    }
                }
            }
        }

        // Search children recursively
        let children_attr = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
        CFRelease(children_attr);

        if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
            let children_count = CFArrayGetCount(children_value);

            for i in 0..children_count {
                let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
                if let Ok(popup) = find_popup_in_element(child, popup_name) {
                    CFRelease(children_value);
                    // popup is already retained by the recursive call
                    return Ok(popup);
                }
            }

            CFRelease(children_value); // ✅ Release after loop
        }

        bail!("Popup not found")
    }
}

/// Uncheck a checkbox (set to unchecked state)
pub fn uncheck_box(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    unsafe {
        super::helpers::with_app_window(app_name, window_name, |_app, window| {
            let checkbox_element = find_checkbox_in_window(window, checkbox_name)?;

            println!(
                "Found checkbox '{}', setting to UNCHECKED...",
                checkbox_name
            );

            // Create a CFNumber for value 0 (unchecked)
            let num_value: i32 = 0;
            let cf_number = CFNumberCreate(
                std::ptr::null(),
                9, // kCFNumberSInt32Type
                &num_value as *const i32 as *const c_void,
            );

            // Set the value
            let value_attr = create_cfstring("AXValue");
            let result = AXUIElementSetAttributeValue(checkbox_element, value_attr, cf_number);
            CFRelease(value_attr);
            CFRelease(cf_number);

            if result != K_AX_ERROR_SUCCESS {
                CFRelease(checkbox_element);
                bail!(
                    "Failed to uncheck checkbox '{}' (error code: {})",
                    checkbox_name,
                    result
                );
            }

            CFRelease(checkbox_element);
            println!("✅ Set checkbox '{}' to UNCHECKED", checkbox_name);
            Ok(())
        })
    }
}
// ============================================================================
// Helper Functions
// ============================================================================

/// Get the currently focused window
pub(crate) unsafe fn get_focused_window(app_element: AXUIElementRef) -> Result<AXUIElementRef> {
    unsafe {
        let attr = create_cfstring("AXFocusedWindow");
        let mut window: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(app_element, attr, &mut window);
        CFRelease(attr);

        if result != K_AX_ERROR_SUCCESS {
            bail!("Failed to get focused window (error: {})", result);
        }

        if window.is_null() {
            bail!("No focused window");
        }

        Ok(window)
    }
}

/// Get all window titles for an application
///
/// Returns a vector of window titles in the order they appear in the window list
///
/// # Arguments
/// * `app_name` - Name of the application
///
/// # Example
/// ```ignore
/// let titles = get_window_titles("iZotope RX")?;
/// // Returns: ["iZotope RX 11 Audio Editor", "iZotope RX 11 Audio Editor"]
/// // (2 windows with same title means render dialog is open)
/// ```
pub fn get_window_titles(app_name: &str) -> Result<Vec<String>> {
    unsafe {
        let pid = get_pid_by_name(app_name)?;
        let app_element = AXUIElementCreateApplication(pid);

        // Get all windows
        let windows_attr = create_cfstring("AXWindows");
        let mut windows_value: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(app_element, windows_attr, &mut windows_value);
        CFRelease(windows_attr);
        CFRelease(app_element);

        if result != K_AX_ERROR_SUCCESS || windows_value.is_null() {
            bail!("Failed to get windows list");
        }

        let windows_count = CFArrayGetCount(windows_value);
        let mut titles = Vec::new();

        // Collect all window titles
        for i in 0..windows_count {
            let window = CFArrayGetValueAtIndex(windows_value, i) as AXUIElementRef;

            let title_attr = create_cfstring("AXTitle");
            let mut title_value: *mut c_void = ptr::null_mut();

            AXUIElementCopyAttributeValue(window, title_attr, &mut title_value);
            CFRelease(title_attr);

            if let Some(title) = cfstring_to_string(title_value) {
                if !title_value.is_null() {
                    CFRelease(title_value);
                }
                titles.push(title);
            }
        }

        CFRelease(windows_value);
        Ok(titles)
    }
}

/// Find a window by name using soft matching
pub(crate) unsafe fn find_window_by_name(
    app_element: AXUIElementRef,
    window_name: &str,
) -> Result<AXUIElementRef> {
    unsafe {
        // Get all windows
        let windows_attr = create_cfstring("AXWindows");
        let mut windows_value: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(app_element, windows_attr, &mut windows_value);
        CFRelease(windows_attr);

        if result != K_AX_ERROR_SUCCESS || windows_value.is_null() {
            bail!("Failed to get windows list");
        }

        let windows_count = CFArrayGetCount(windows_value);

        // Search for window with matching title
        for i in 0..windows_count {
            let window = CFArrayGetValueAtIndex(windows_value, i) as AXUIElementRef;

            let title_attr = create_cfstring("AXTitle");
            let mut title_value: *mut c_void = ptr::null_mut();

            AXUIElementCopyAttributeValue(window, title_attr, &mut title_value);
            CFRelease(title_attr);

            if let Some(title) = cfstring_to_string(title_value) {
                if !title_value.is_null() {
                    CFRelease(title_value);
                }

                // Use soft_match from main.rs
                if crate::soft_match(&title, window_name) {
                    // Found it! Retain the window before returning (CFArrayGetValueAtIndex returns non-retained)
                    CFRetain(window);
                    CFRelease(windows_value);
                    return Ok(window);
                }
            }
        }

        CFRelease(windows_value);
        bail!("Window '{}' not found", window_name)
    }
}

/// Find all buttons in a UI element (recursive)
unsafe fn find_buttons_in_element(element: AXUIElementRef) -> Result<Vec<String>> {
    unsafe {
        let mut buttons = Vec::new();

        // Get role
        let role_attr = create_cfstring("AXRole");
        let mut role_value: *mut c_void = ptr::null_mut();
        AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
        CFRelease(role_attr);

        if let Some(role) = cfstring_to_string(role_value) {
            if !role_value.is_null() {
                CFRelease(role_value);
            }

            // If this is a button, get its title
            if role == "AXButton" {
                let title_attr = create_cfstring("AXTitle");
                let mut title_value: *mut c_void = ptr::null_mut();
                AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
                CFRelease(title_attr);

                if let Some(title) = cfstring_to_string(title_value) {
                    if !title_value.is_null() {
                        CFRelease(title_value);
                    }
                    if !title.is_empty() {
                        buttons.push(title);
                    }
                }
            }
        }

        // Recursively search children
        let children_attr = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
        CFRelease(children_attr);

        if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
            let children_count = CFArrayGetCount(children_value);

            for i in 0..children_count {
                let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
                if let Ok(mut child_buttons) = find_buttons_in_element(child) {
                    buttons.append(&mut child_buttons);
                }
            }

            CFRelease(children_value); // ✅ Release after loop
        }

        Ok(buttons)
    }
}

/// Find a button in a window by name (soft matched)
unsafe fn find_button_in_window(
    window: AXUIElementRef,
    button_name: &str,
) -> Result<AXUIElementRef> {
    unsafe {
        find_button_in_element(window, button_name)
            .with_context(|| format!("Button '{}' not found in window", button_name))
    }
}

/// Recursively find a button by name
unsafe fn find_button_in_element(
    element: AXUIElementRef,
    button_name: &str,
) -> Result<AXUIElementRef> {
    unsafe {
        // Check if this element is a button with matching name
        let role_attr = create_cfstring("AXRole");
        let mut role_value: *mut c_void = ptr::null_mut();
        AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
        CFRelease(role_attr);

        if let Some(role) = cfstring_to_string(role_value) {
            if !role_value.is_null() {
                CFRelease(role_value);
            }

            if role == "AXButton" {
                let title_attr = create_cfstring("AXTitle");
                let mut title_value: *mut c_void = ptr::null_mut();
                AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
                CFRelease(title_attr);

                if let Some(title) = cfstring_to_string(title_value) {
                    if !title_value.is_null() {
                        CFRelease(title_value);
                    }

                    if crate::soft_match(&title, button_name) {
                        // Found it! Retain before returning so caller owns it
                        CFRetain(element);
                        return Ok(element);
                    }
                }
            }
        }

        // Search children recursively
        let children_attr = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
        CFRelease(children_attr);

        if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
            let children_count = CFArrayGetCount(children_value);

            for i in 0..children_count {
                let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
                if let Ok(button) = find_button_in_element(child, button_name) {
                    CFRelease(children_value);
                    // button is already retained by the recursive call
                    return Ok(button);
                }
            }

            CFRelease(children_value); // ✅ Release after loop
        }

        bail!("Button not found")
    }
}

/// Find a checkbox in a window by name (soft matched)
unsafe fn find_checkbox_in_window(
    window: AXUIElementRef,
    checkbox_name: &str,
) -> Result<AXUIElementRef> {
    unsafe {
        find_checkbox_in_element(window, checkbox_name)
            .with_context(|| format!("Checkbox '{}' not found in window", checkbox_name))
    }
}

/// Recursively find a checkbox by name
unsafe fn find_checkbox_in_element(
    element: AXUIElementRef,
    checkbox_name: &str,
) -> Result<AXUIElementRef> {
    unsafe {
        // Check if this element is a checkbox with matching name
        let role_attr = create_cfstring("AXRole");
        let mut role_value: *mut c_void = ptr::null_mut();
        AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
        CFRelease(role_attr);

        if let Some(role) = cfstring_to_string(role_value) {
            if !role_value.is_null() {
                CFRelease(role_value);
            }

            if role == "AXCheckBox" {
                let title_attr = create_cfstring("AXTitle");
                let mut title_value: *mut c_void = ptr::null_mut();
                AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
                CFRelease(title_attr);

                if let Some(title) = cfstring_to_string(title_value) {
                    if !title_value.is_null() {
                        CFRelease(title_value);
                    }

                    if crate::soft_match(&title, checkbox_name) {
                        // Found it! Retain before returning so caller owns it
                        CFRetain(element);
                        return Ok(element);
                    }
                }
            }
        }

        // Search children recursively
        let children_attr = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
        CFRelease(children_attr);

        if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
            let children_count = CFArrayGetCount(children_value);

            for i in 0..children_count {
                let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
                if let Ok(checkbox) = find_checkbox_in_element(child, checkbox_name) {
                    CFRelease(children_value);
                    // checkbox is already retained by the recursive call
                    return Ok(checkbox);
                }
            }

            CFRelease(children_value); // ✅ Release after loop
        }

        bail!("Checkbox not found")
    }
}

///
/// Check if a window exists right now
///
/// # Returns
/// Ok(true) if window exists, Ok(false) if not found
pub fn window_exists(app_name: &str, window_name: &str) -> Result<bool> {
    unsafe {
        let pid = get_pid_by_name(app_name)?;
        let app_element = AXUIElementCreateApplication(pid);

        // find_window_by_name returns a retained window that needs CFRelease
        let exists = match find_window_by_name(app_element, window_name) {
            Ok(window) => {
                CFRelease(window); // ✅ Release the window element
                true
            }
            Err(_) => false,
        };

        CFRelease(app_element);

        Ok(exists)
    }
}

/// Wait for a window to appear
///
/// Polls every 100ms until window is found or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for (soft matched)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_exists(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(100);

    log::info!(
        "Waiting for window '{}' in '{}' (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    loop {
        if window_exists(app_name, window_name)? {
            log::info!("✅ Window '{}' appeared", window_name);
            return Ok(());
        }

        if start.elapsed() >= timeout {
            bail!(
                "Timeout waiting for window '{}' ({}ms)",
                window_name,
                timeout_ms
            );
        }

        std::thread::sleep(poll_interval);
    }
}

/// Wait for a window to disappear/hide
///
/// Polls every 50ms until window is gone or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for to disappear (soft matched)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_closed(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(50);

    println!(
        "⏳ Waiting for window '{}' in '{}' to close (timeout: {}ms)",
        window_name, app_name, timeout_ms
    );
    log::info!(
        "Waiting for window '{}' in '{}' to close (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    loop {
        // Check if window still exists
        let exists = window_exists(app_name, window_name)?;
        println!(
            "🔍 Checking if window '{}' in '{}' still exists: {}",
            window_name, app_name, exists
        );

        if !exists {
            println!("✅ Window '{}' has closed", window_name);
            log::info!("✅ Window '{}' has closed", window_name);
            return Ok(());
        } else {
            println!("⏳ Window '{}' still exists, waiting...", window_name);
        }

        if start.elapsed() >= timeout {
            println!(
                "❌ Timeout waiting for window '{}' to close ({}ms)",
                window_name, timeout_ms
            );
            bail!(
                "Timeout waiting for window '{}' to close ({}ms)",
                window_name,
                timeout_ms
            );
        }

        std::thread::sleep(poll_interval);
    }
}

/// Wait for a window to become focused
///
/// Polls every 100ms until the specified window becomes the focused window or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for to be focused (soft matched).
///   If empty string, just waits for the app to be focused (any window)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_focused(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(50);

    if window_name.is_empty() {
        log::info!(
            "Waiting for '{}' to be focused (timeout: {}ms)",
            app_name,
            timeout_ms
        );
    } else {
        log::info!(
            "Waiting for window '{}' in '{}' to be focused (timeout: {}ms)",
            window_name,
            app_name,
            timeout_ms
        );
    }

    // Try to focus the application first
    println!("🎯 Attempting to focus '{}'...", app_name);
    if let Err(e) = crate::macos::app_info::focus_application(app_name) {
        println!("⚠️  Failed to focus app: {}", e);
        log::warn!("Failed to focus app '{}': {}", app_name, e);
    }

    loop {
        // Sleep first, then check
        std::thread::sleep(poll_interval);

        // Check if the specified app is the current app
        if let Ok(current_app) = crate::macos::app_info::get_current_app() {
            println!(
                "🔍 Current app: '{}', looking for: '{}'",
                current_app, app_name
            );
            log::debug!(
                "Current app: '{}', looking for: '{}'",
                current_app,
                app_name
            );

            if crate::soft_match(&current_app, app_name) {
                println!("✅ App matches!");
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
                if let Ok(current_window) = crate::macos::app_info::get_app_window() {
                    println!(
                        "🔍 Current window: '{}', looking for: '{}'",
                        current_window, window_name
                    );
                    log::debug!(
                        "Current window: '{}', looking for: '{}'",
                        current_window,
                        window_name
                    );
                    if crate::soft_match(&current_window, window_name) {
                        println!("✅ Window '{}' is now focused", window_name);
                        log::info!("✅ Window '{}' is now focused", window_name);
                        return Ok(());
                    } else {
                        println!("❌ Window doesn't match");
                    }
                } else {
                    println!("❌ Failed to get current window title");
                    log::debug!("Failed to get current window title");
                }
            } else {
                println!("❌ App doesn't match");
            }
        } else {
            println!("❌ Failed to get current app");
            log::debug!("Failed to get current app");
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
    }
}

pub fn close_window(app_name: &str, window_name: &str) -> Result<()> {
    unsafe {
        let pid = get_pid_by_name(app_name)?;
        let app_element = AXUIElementCreateApplication(pid);

        let window = if window_name.is_empty() {
            get_focused_window(app_element)?
        } else {
            find_window_by_name(app_element, window_name)?
        };

        // Get the close button (standard AXCloseButton)
        let close_attr = create_cfstring("AXCloseButton");
        let mut close_button: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(window, close_attr, &mut close_button);
        CFRelease(close_attr);

        if result != K_AX_ERROR_SUCCESS || close_button.is_null() {
            CFRelease(window);
            CFRelease(app_element);
            bail!("Window does not have a close button");
        }

        // Click the close button
        let press_action = create_cfstring("AXPress");
        let result = AXUIElementPerformAction(close_button, press_action);
        CFRelease(press_action);
        CFRelease(close_button);
        CFRelease(window);
        CFRelease(app_element);

        if result != K_AX_ERROR_SUCCESS {
            bail!("Failed to click close button");
        }

        log::info!(
            "✅ Closed window '{}'",
            if window_name.is_empty() {
                "<focused>"
            } else {
                window_name
            }
        );

        Ok(())
    }
}
/// Close a window with retry - keeps trying until window disappears
///
/// Useful when window might not close immediately (e.g., during rendering)
/// Retries every 500ms until window is gone or timeout reached
/// Close window with retry - keeps trying until window is gone
pub fn close_window_with_retry(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    loop {
        // Check if already closed
        if !window_exists(app_name, window_name)? {
            log::info!("✅ Window '{}' closed", window_name);
            return Ok(());
        }

        // Try to close it
        close_window(app_name, window_name).ok(); // Ignore errors, will retry

        // Wait a bit before checking again
        std::thread::sleep(Duration::from_millis(100));

        if start.elapsed() >= timeout {
            bail!(
                "Timeout trying to close window '{}' ({}ms)",
                window_name,
                timeout_ms
            );
        }
    }
}

/// Get all text content from a window
///
/// Recursively searches for AXStaticText elements and collects their values
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of the window, or "" for focused window
///
/// # Returns
/// Vector of text strings found in the window
///
/// # Example
/// ```ignore
/// let text = get_window_text("Pro Tools", "AudioSuite: Reverb")?;
/// // Returns: ["Press RENDER to commit changes", "Render", "Cancel", ...]
/// ```
pub fn get_window_text(app_name: &str, window_name: &str) -> Result<Vec<String>> {
    unsafe {
        let pid = get_pid_by_name(app_name)?;
        let app_element = AXUIElementCreateApplication(pid);

        // Get the target window
        let window = if window_name.is_empty() {
            get_focused_window(app_element)?
        } else {
            find_window_by_name(app_element, window_name)?
        };

        // Get all text in the window
        let text_elements = find_text_in_element(window)?;

        // Clean up
        CFRelease(app_element);
        CFRelease(window);

        Ok(text_elements)
    }
}

/// Find all text-containing elements recursively
/// Returns strings in format "Role: Text" so user can see what element type contains the text
unsafe fn find_text_in_element(element: AXUIElementRef) -> Result<Vec<String>> {
    unsafe {
        let mut text_strings = Vec::new();

        // Get role
        let role_attr = create_cfstring("AXRole");
        let mut role_value: *mut c_void = ptr::null_mut();
        let role_result = AXUIElementCopyAttributeValue(element, role_attr, &mut role_value);
        CFRelease(role_attr);

        let role = if role_result == K_AX_ERROR_SUCCESS && !role_value.is_null() {
            let r = cfstring_to_string(role_value).unwrap_or_else(|| "Unknown".to_string());
            CFRelease(role_value);
            r
        } else {
            "Unknown".to_string()
        };

        // Try to get AXValue from ANY element (not just specific roles)
        let value_attr = create_cfstring("AXValue");
        let mut value: *mut c_void = ptr::null_mut();
        let value_result = AXUIElementCopyAttributeValue(element, value_attr, &mut value);
        CFRelease(value_attr);

        if value_result == K_AX_ERROR_SUCCESS && !value.is_null() {
            // cfstring_to_string now safely checks if value is a CFString
            if let Some(text) = cfstring_to_string(value)
                && !text.is_empty()
            {
                // Include the role type so user can see what contains the text
                text_strings.push(format!("[{}] {}", role, text));
            }
            CFRelease(value);
        }

        // Also try AXTitle attribute (some elements use this for text)
        let title_attr = create_cfstring("AXTitle");
        let mut title_value: *mut c_void = ptr::null_mut();
        let title_result = AXUIElementCopyAttributeValue(element, title_attr, &mut title_value);
        CFRelease(title_attr);

        if title_result == K_AX_ERROR_SUCCESS && !title_value.is_null() {
            if let Some(text) = cfstring_to_string(title_value)
                && !text.is_empty()
                && !text_strings.iter().any(|s| s.contains(&text))
            {
                text_strings.push(format!("[{} Title] {}", role, text));
            }
            CFRelease(title_value);
        }

        // Also try AXDescription attribute
        let desc_attr = create_cfstring("AXDescription");
        let mut desc_value: *mut c_void = ptr::null_mut();
        let desc_result = AXUIElementCopyAttributeValue(element, desc_attr, &mut desc_value);
        CFRelease(desc_attr);

        if desc_result == K_AX_ERROR_SUCCESS && !desc_value.is_null() {
            if let Some(text) = cfstring_to_string(desc_value)
                && !text.is_empty()
                && !text_strings.iter().any(|s| s.contains(&text))
            {
                text_strings.push(format!("[{} Description] {}", role, text));
            }
            CFRelease(desc_value);
        }

        // Recursively search children
        let children_attr = create_cfstring("AXChildren");
        let mut children_value: *mut c_void = ptr::null_mut();
        let result = AXUIElementCopyAttributeValue(element, children_attr, &mut children_value);
        CFRelease(children_attr);

        if result == K_AX_ERROR_SUCCESS && !children_value.is_null() {
            let children_count = CFArrayGetCount(children_value);

            for i in 0..children_count {
                let child = CFArrayGetValueAtIndex(children_value, i) as AXUIElementRef;
                if let Ok(mut child_text) = find_text_in_element(child) {
                    text_strings.append(&mut child_text);
                }
            }

            CFRelease(children_value);
        }

        Ok(text_strings)
    }
}
