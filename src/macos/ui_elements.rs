//! UI element automation for macOS
//!
//! Provides functions to find and interact with UI elements (buttons, etc.)
//! in application windows using the macOS Accessibility API.
//!
//! **Thread Safety**: All public functions automatically dispatch to the main
//! thread using Grand Central Dispatch, as required by the macOS Accessibility API.

use super::ffi::*;
use super::helpers::{CFArray, CFNumber};
use anyhow::{Context, Result, bail};
use libc::c_void;
use std::ptr;

// ============================================================================
// Thread Safety Helper
// ============================================================================

/// Helper to dispatch a closure to the main thread and wait for result
///
/// All Accessibility API calls must run on the main thread. This helper
/// uses Grand Central Dispatch to ensure thread safety.
unsafe fn dispatch_to_main<F, R>(f: F) -> Result<R>
where
    F: FnOnce() -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    unsafe {
        use std::sync::mpsc;
        use std::time::Duration;
        let (tx, rx) = mpsc::channel();

        super::events::dispatch_to_main_queue(move || {
            // Catch panics to prevent wedging the main queue
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

            let final_result = match result {
                Ok(r) => r,
                Err(panic_info) => {
                    log::error!("UI operation PANICKED: {:?}", panic_info);
                    Err(anyhow::anyhow!("UI operation panicked"))
                }
            };

            let _ = tx.send(final_result);
        });

        rx.recv_timeout(Duration::from_secs(5)).map_err(|e| {
            anyhow::anyhow!("UI operation timed out: {}. Main thread may be blocked.", e)
        })?
    }
}

/// Get all buttons in a window
///
/// **Thread Safety**: Automatically dispatches to main thread.
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
    // Use Swift bridge instead of direct AX API to avoid crashes
    crate::swift_bridge::get_window_buttons(app_name, window_name)
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
    // Use Swift bridge instead of direct AX API to avoid crashes
    crate::swift_bridge::click_button(app_name, window_name, button_name)?;

    log::info!(
        "✅ Clicked button '{}' in window '{}' of app '{}'",
        button_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        },
        if app_name.is_empty() {
            "<frontmost>"
        } else {
            app_name
        }
    );

    Ok(())
}
pub fn click_checkbox(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    // Use Swift bridge instead of direct AX API to avoid crashes
    crate::swift_bridge::click_checkbox(app_name, window_name, checkbox_name)?;

    log::info!(
        "✅ Clicked checkbox '{}' in window '{}' of app '{}'",
        checkbox_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            window_name
        },
        if app_name.is_empty() {
            "<frontmost>"
        } else {
            app_name
        }
    );

    Ok(())
}

/// Check a checkbox (set to checked state)
pub fn check_box(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    let app_name = app_name.to_string();
    let window_name = window_name.to_string();
    let checkbox_name = checkbox_name.to_string();

    unsafe {
        dispatch_to_main(move || {
            super::helpers::with_app_window(&app_name, &window_name, |_app, window| {
                use super::session::MacOSSession;

                let checkbox_element = find_checkbox_in_window(window, &checkbox_name)?;

                println!("Found checkbox '{}', setting to CHECKED...", checkbox_name);

                // Create a CFNumber for value 1 (checked)
                let cf_number = CFNumber::from_i32(1);

                // Set the value using building block
                let os = MacOSSession::global();
                if let Err(e) = os.set_ax_attribute(checkbox_element, "AXValue", cf_number.as_ptr())
                {
                    CFRelease(checkbox_element);
                    return Err(e);
                }

                CFRelease(checkbox_element);
                println!("✅ Set checkbox '{}' to CHECKED", checkbox_name);
                Ok(())
            })
        })
    }
}

/// Get menu items from a popup button
pub fn get_popup_menu_items(
    app_name: &str,
    window_name: &str,
    popup_name: &str,
) -> Result<Vec<String>> {
    let app_name = app_name.to_string();
    let window_name = window_name.to_string();
    let popup_name = popup_name.to_string();

    unsafe {
        dispatch_to_main(move || {
            super::helpers::with_app_window(&app_name, &window_name, |_app, window| {
                use super::session::MacOSSession;
                let os = MacOSSession::global();

                // Find the popup button
                let popup_element = find_popup_in_window(window, &popup_name)?;

                println!("Found popup '{}', opening menu...", popup_name);

                // Click it to open the menu
                let result = os.perform_ax_action(popup_element, "AXPress");

                // Note: Some apps (like Pro Tools) return error codes even though the popup opens
                // For example, Pro Tools returns -25204 (K_AX_ERROR_INVALID_UI_ELEMENT) but the popup still opens
                if result.is_err() {
                    println!("  ⚠️  AXPress returned error (but popup may still open)");
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
                if let Ok(menu_value) = os.get_ax_element_attr(popup_element, "AXMenu") {
                    println!("  Found menu via AXMenu attribute!");
                    menu_items = get_menu_items_from_menu(menu_value)?;
                    CFRelease(menu_value);
                } else {
                    println!("  No AXMenu attribute, checking children...");

                    // Try method 2: Look in children
                    if let Ok(children_value) = os.get_ax_element_attr(popup_element, "AXChildren")
                    {
                        let children = CFArray::new(children_value);
                        println!("  Found {} children", children.count());

                        for i in 0..children.count() {
                            let child = children.get(i) as AXUIElementRef;

                            // Get the role of this child
                            if let Ok(role) = os.get_ax_string_attr(child, "AXRole") {
                                println!("    Child {}: role = {}", i, role);

                                // Look for menu items
                                if role == "AXMenu" {
                                    println!("  Found AXMenu child!");
                                    menu_items = get_menu_items_from_menu(child)?;
                                    break;
                                }
                            }
                        }
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
        })
    }
}

/// Get menu item titles from a menu element
unsafe fn get_menu_items_from_menu(menu: AXUIElementRef) -> Result<Vec<String>> {
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        let mut items = Vec::new();

        if let Ok(children_value) = os.get_ax_element_attr(menu, "AXChildren") {
            let children = CFArray::new(children_value);

            for i in 0..children.count() {
                let child = children.get(i) as AXUIElementRef;

                // Get the title of this menu item
                if let Ok(title) = os.get_ax_string_attr(child, "AXTitle")
                    && !title.is_empty()
                {
                    items.push(title);
                }
            }
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
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        // Check if this element is a popup button with matching name
        if let Ok(role) = os.get_ax_string_attr(element, "AXRole")
            && role == "AXPopUpButton"
            && let Ok(title) = os.get_ax_string_attr(element, "AXTitle")
            && crate::soft_match(&title, popup_name)
        {
            CFRetain(element);
            return Ok(element);
        }

        // Search children recursively
        if let Ok(children_value) = os.get_ax_element_attr(element, "AXChildren") {
            let children = CFArray::new(children_value);

            for i in 0..children.count() {
                let child = children.get(i) as AXUIElementRef;
                if let Ok(popup) = find_popup_in_element(child, popup_name) {
                    // popup is already retained by the recursive call
                    return Ok(popup);
                }
            }
        }

        bail!("Popup not found")
    }
}

/// Uncheck a checkbox (set to unchecked state)
pub fn uncheck_box(app_name: &str, window_name: &str, checkbox_name: &str) -> Result<()> {
    let app_name = app_name.to_string();
    let window_name = window_name.to_string();
    let checkbox_name = checkbox_name.to_string();

    unsafe {
        dispatch_to_main(move || {
            super::helpers::with_app_window(&app_name, &window_name, |_app, window| {
                use super::session::MacOSSession;

                let checkbox_element = find_checkbox_in_window(window, &checkbox_name)?;

                println!(
                    "Found checkbox '{}', setting to UNCHECKED...",
                    checkbox_name
                );

                // Create a CFNumber for value 0 (unchecked)
                let cf_number = CFNumber::from_i32(0);

                // Set the value using building block
                let os = MacOSSession::global();
                if let Err(e) = os.set_ax_attribute(checkbox_element, "AXValue", cf_number.as_ptr())
                {
                    CFRelease(checkbox_element);
                    return Err(e);
                }

                CFRelease(checkbox_element);
                println!("✅ Set checkbox '{}' to UNCHECKED", checkbox_name);
                Ok(())
            })
        })
    }
}
// ============================================================================
// Helper Functions
// ============================================================================

/// Get the currently focused window
pub(crate) unsafe fn get_focused_window(app_element: AXUIElementRef) -> Result<AXUIElementRef> {
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        os.get_ax_element_attr(app_element, "AXFocusedWindow")
            .context("Failed to get focused window")
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
    crate::swift_bridge::get_window_titles(app_name)
}

/// Find a window by name using soft matching
pub(crate) unsafe fn find_window_by_name(
    app_element: AXUIElementRef,
    window_name: &str,
) -> Result<AXUIElementRef> {
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        // Get all windows using building block
        let windows_value = os.get_ax_element_attr(app_element, "AXWindows")?;
        let windows = CFArray::new(windows_value);

        // Search for window with matching title
        for i in 0..windows.count() {
            let window = windows.get(i) as AXUIElementRef;

            if let Ok(title) = os.get_ax_string_attr(window, "AXTitle") {
                // Use soft_match from main.rs
                if crate::soft_match(&title, window_name) {
                    // Found it! Retain the window before returning (CFArrayGetValueAtIndex returns non-retained)
                    CFRetain(window);
                    return Ok(window);
                }
            }
        }

        bail!("Window '{}' not found", window_name)
    }
}

// Old button finding functions removed - now using Swift bridge via:
// - crate::swift_bridge::click_button()
// - crate::swift_bridge::click_checkbox()
// - crate::swift_bridge::get_window_buttons()
//
// Note: checkbox finding functions below are still used by check_box/uncheck_box
// which set specific values (not just click/toggle)

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
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        // Check if this element is a checkbox with matching name
        if let Ok(role) = os.get_ax_string_attr(element, "AXRole")
            && role == "AXCheckBox"
            && let Ok(title) = os.get_ax_string_attr(element, "AXTitle")
            && crate::soft_match(&title, checkbox_name)
        {
            // Found it! Retain before returning so caller owns it
            CFRetain(element);
            return Ok(element);
        }

        // Search children recursively
        if let Ok(children_value) = os.get_ax_element_attr(element, "AXChildren") {
            let children = CFArray::new(children_value);

            for i in 0..children.count() {
                let child = children.get(i) as AXUIElementRef;
                if let Ok(checkbox) = find_checkbox_in_element(child, checkbox_name) {
                    // checkbox is already retained by the recursive call
                    return Ok(checkbox);
                }
            }
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
    crate::swift_bridge::window_exists(app_name, window_name)
}

/// Wait for a window to appear
///
/// Polls every 50ms until window is found or timeout is reached
///
/// # Arguments
/// * `app_name` - Name of the application
/// * `window_name` - Name of window to wait for (soft matched)
/// * `timeout_ms` - Maximum time to wait in milliseconds
pub fn wait_for_window_exists(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    log::info!(
        "Waiting for window '{}' in '{}' (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    if crate::swift_bridge::wait_for_window(
        app_name,
        window_name,
        crate::swift_bridge::WindowCondition::Exists,
        timeout_ms as i32,
    )? {
        log::info!("✅ Window '{}' appeared", window_name);
        Ok(())
    } else {
        bail!(
            "Timeout waiting for window '{}' ({}ms)",
            window_name,
            timeout_ms
        )
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
    log::info!(
        "Waiting for window '{}' in '{}' to close (timeout: {}ms)",
        window_name,
        app_name,
        timeout_ms
    );

    if crate::swift_bridge::wait_for_window(
        app_name,
        window_name,
        crate::swift_bridge::WindowCondition::Closed,
        timeout_ms as i32,
    )? {
        log::info!("✅ Window '{}' has closed", window_name);
        Ok(())
    } else {
        bail!(
            "Timeout waiting for window '{}' to close ({}ms)",
            window_name,
            timeout_ms
        )
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

    if crate::swift_bridge::wait_for_window(
        app_name,
        window_name,
        crate::swift_bridge::WindowCondition::Focused,
        timeout_ms as i32,
    )? {
        if window_name.is_empty() {
            log::info!("✅ '{}' is now focused", app_name);
        } else {
            log::info!("✅ Window '{}' is now focused", window_name);
        }
        Ok(())
    } else if window_name.is_empty() {
        bail!(
            "Timeout waiting for '{}' to be focused ({}ms)",
            app_name,
            timeout_ms
        )
    } else {
        bail!(
            "Timeout waiting for window '{}' to be focused ({}ms)",
            window_name,
            timeout_ms
        )
    }
}

pub fn close_window(app_name: &str, window_name: &str) -> Result<()> {
    crate::swift_bridge::close_window(app_name, window_name, None)?;
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
/// Close a window with retry - keeps trying until window disappears
///
/// Useful when window might not close immediately (e.g., during rendering)
/// Retries every 500ms until window is gone or timeout reached
/// Close window with retry - keeps trying until window is gone
pub fn close_window_with_retry(app_name: &str, window_name: &str, timeout_ms: u64) -> Result<()> {
    crate::swift_bridge::close_window(app_name, window_name, Some(timeout_ms as i32))?;
    log::info!("✅ Window '{}' closed", window_name);
    Ok(())
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
        super::helpers::with_app_window(app_name, window_name, |_app, window| {
            find_text_in_element(window)
        })
    }
}

/// Find all text-containing elements recursively
/// Returns strings in format "Role: Text" so user can see what element type contains the text
unsafe fn find_text_in_element(element: AXUIElementRef) -> Result<Vec<String>> {
    use super::session::MacOSSession;
    let os = MacOSSession::global();

    unsafe {
        let mut text_strings = Vec::new();

        // Get role using building block
        let role = os
            .get_ax_string_attr(element, "AXRole")
            .unwrap_or_else(|_| "Unknown".to_string());

        // Try to get AXValue from ANY element (not just specific roles)
        // Note: We can't use get_ax_string_attr here because AXValue might not be a CFString
        // So we keep the manual approach for AXValue
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
        if let Ok(text) = os.get_ax_string_attr(element, "AXTitle")
            && !text.is_empty()
            && !text_strings.iter().any(|s| s.contains(&text))
        {
            text_strings.push(format!("[{} Title] {}", role, text));
        }

        // Also try AXDescription attribute
        if let Ok(text) = os.get_ax_string_attr(element, "AXDescription")
            && !text.is_empty()
            && !text_strings.iter().any(|s| s.contains(&text))
        {
            text_strings.push(format!("[{} Description] {}", role, text));
        }

        // Recursively search children
        if let Ok(children_value) = os.get_ax_element_attr(element, "AXChildren") {
            let children = CFArray::new(children_value);

            for i in 0..children.count() {
                let child = children.get(i) as AXUIElementRef;
                if let Ok(mut child_text) = find_text_in_element(child) {
                    text_strings.append(&mut child_text);
                }
            }
        }

        Ok(text_strings)
    }
}
