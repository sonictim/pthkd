//! UI element interaction implementation for MacOSSession
//!
//! Functions for clicking buttons, getting window text, etc.

use super::session::MacOSSession;
use super::ffi;
use anyhow::{Result, bail};
use libc::c_void;
use std::ptr;

impl MacOSSession {
    /// Click a button in a window
    pub async fn click_button(
        &mut self,
        app_name: &str,
        window_name: &str,
        button_name: &str,
    ) -> Result<()> {
        let pid = self.get_pid(app_name)?;

        unsafe {
            let app_element = ffi::create_app_element(pid)?;

            // Get the target window
            let window = if window_name.is_empty() {
                get_focused_window(app_element)?
            } else {
                find_window_by_name(app_element, window_name)?
            };

            // Find the button
            let button_element = find_button_in_window(window, button_name)?;

            // Click it
            let press_action = ffi::create_cfstring("AXPress");
            let result = ffi::AXUIElementPerformAction(button_element, press_action);
            ffi::CFRelease(press_action);

            if result != ffi::K_AX_ERROR_SUCCESS {
                ffi::CFRelease(button_element);
                ffi::CFRelease(window);
                ffi::CFRelease(app_element);
                bail!("Failed to press button '{}' (error code: {})", button_name, result);
            }

            // Clean up
            ffi::CFRelease(button_element);
            ffi::CFRelease(window);
            ffi::CFRelease(app_element);

            log::info!(
                "✅ Clicked button '{}' in window '{}'",
                button_name,
                if window_name.is_empty() { "<focused>" } else { window_name }
            );

            Ok(())
        }
    }

    /// Get all button names in a window
    pub async fn get_window_buttons(
        &mut self,
        app_name: &str,
        window_name: &str,
    ) -> Result<Vec<String>> {
        let pid = self.get_pid(app_name)?;

        unsafe {
            let app_element = ffi::create_app_element(pid)?;

            let window = if window_name.is_empty() {
                get_focused_window(app_element)?
            } else {
                find_window_by_name(app_element, window_name)?
            };

            let buttons = get_all_buttons_in_window(window)?;

            ffi::CFRelease(window);
            ffi::CFRelease(app_element);

            Ok(buttons)
        }
    }

    /// Get all static text elements in a window
    pub async fn get_window_text(
        &mut self,
        app_name: &str,
        window_name: &str,
    ) -> Result<Vec<String>> {
        let pid = self.get_pid(app_name)?;

        unsafe {
            let app_element = ffi::create_app_element(pid)?;

            let window = if window_name.is_empty() {
                get_focused_window(app_element)?
            } else {
                find_window_by_name(app_element, window_name)?
            };

            let text = get_all_text_in_window(window)?;

            ffi::CFRelease(window);
            ffi::CFRelease(app_element);

            Ok(text)
        }
    }

    /// Check if a window exists
    pub async fn window_exists(
        &mut self,
        app_name: &str,
        window_name: &str,
    ) -> Result<bool> {
        let pid = self.get_pid(app_name)?;

        unsafe {
            let app_element = ffi::create_app_element(pid)?;
            let result = find_window_by_name(app_element, window_name);

            if let Ok(window) = result {
                ffi::CFRelease(window);
                ffi::CFRelease(app_element);
                Ok(true)
            } else {
                ffi::CFRelease(app_element);
                Ok(false)
            }
        }
    }

    /// Close a window
    pub async fn close_window(
        &mut self,
        app_name: &str,
        window_name: &str,
    ) -> Result<()> {
        let pid = self.get_pid(app_name)?;

        unsafe {
            let app_element = ffi::create_app_element(pid)?;
            let window = find_window_by_name(app_element, window_name)?;

            // Look for close button
            let close_button_attr = ffi::create_cfstring("AXCloseButton");
            let mut close_button: *mut c_void = ptr::null_mut();
            let result = ffi::AXUIElementCopyAttributeValue(
                window,
                close_button_attr,
                &mut close_button,
            );
            ffi::CFRelease(close_button_attr);

            if result != ffi::K_AX_ERROR_SUCCESS || close_button.is_null() {
                ffi::CFRelease(window);
                ffi::CFRelease(app_element);
                bail!("Window '{}' has no close button", window_name);
            }

            // Click close button
            let press_action = ffi::create_cfstring("AXPress");
            let result = ffi::AXUIElementPerformAction(close_button as ffi::AXUIElementRef, press_action);
            ffi::CFRelease(press_action);
            ffi::CFRelease(close_button);
            ffi::CFRelease(window);
            ffi::CFRelease(app_element);

            if result != ffi::K_AX_ERROR_SUCCESS {
                bail!("Failed to close window '{}' (error: {})", window_name, result);
            }

            log::info!("✅ Closed window '{}'", window_name);
            Ok(())
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the currently focused window
unsafe fn get_focused_window(app_element: ffi::AXUIElementRef) -> Result<ffi::AXUIElementRef> {
    let attr = ffi::create_cfstring("AXFocusedWindow");
    let mut window: *mut c_void = ptr::null_mut();

    let result = ffi::AXUIElementCopyAttributeValue(app_element, attr, &mut window);
    ffi::CFRelease(attr);

    if result != ffi::K_AX_ERROR_SUCCESS {
        bail!("Failed to get focused window (error: {})", result);
    }

    if window.is_null() {
        bail!("No focused window");
    }

    Ok(window)
}

/// Find a window by name using soft matching
unsafe fn find_window_by_name(
    app_element: ffi::AXUIElementRef,
    window_name: &str,
) -> Result<ffi::AXUIElementRef> {
    let windows_attr = ffi::create_cfstring("AXWindows");
    let mut windows_value: *mut c_void = ptr::null_mut();

    let result = ffi::AXUIElementCopyAttributeValue(app_element, windows_attr, &mut windows_value);
    ffi::CFRelease(windows_attr);

    if result != ffi::K_AX_ERROR_SUCCESS || windows_value.is_null() {
        bail!("Failed to get windows list");
    }

    let windows_count = ffi::CFArrayGetCount(windows_value);

    for i in 0..windows_count {
        let window = ffi::CFArrayGetValueAtIndex(windows_value, i) as ffi::AXUIElementRef;

        if let Ok(Some(title)) = ffi::get_ax_string_attribute(window, "AXTitle") {
            if crate::soft_match(&title, window_name) {
                ffi::CFRelease(windows_value);
                return Ok(window);
            }
        }
    }

    ffi::CFRelease(windows_value);
    bail!("Window '{}' not found", window_name)
}

/// Find a button in a window by name
unsafe fn find_button_in_window(
    window: ffi::AXUIElementRef,
    button_name: &str,
) -> Result<ffi::AXUIElementRef> {
    collect_buttons_recursive(window, button_name, true)
}

/// Get all buttons in a window
unsafe fn get_all_buttons_in_window(window: ffi::AXUIElementRef) -> Result<Vec<String>> {
    let mut buttons = Vec::new();
    collect_buttons_list_recursive(window, &mut buttons)?;
    Ok(buttons)
}

/// Recursively collect buttons (search mode)
unsafe fn collect_buttons_recursive(
    element: ffi::AXUIElementRef,
    target_name: &str,
    is_first_match: bool,
) -> Result<ffi::AXUIElementRef> {
    // Check if this element is a button
    if let Ok(Some(role)) = ffi::get_ax_string_attribute(element, "AXRole") {
        if role == "AXButton" {
            if let Ok(Some(title)) = ffi::get_ax_string_attribute(element, "AXTitle") {
                if crate::soft_match(&title, target_name) {
                    return Ok(element);
                }
            }
        }
    }

    // Get children
    let children_attr = ffi::create_cfstring("AXChildren");
    let mut children: *mut c_void = ptr::null_mut();
    let result = ffi::AXUIElementCopyAttributeValue(element, children_attr, &mut children);
    ffi::CFRelease(children_attr);

    if result != ffi::K_AX_ERROR_SUCCESS || children.is_null() {
        bail!("Button '{}' not found", target_name);
    }

    let count = ffi::CFArrayGetCount(children);
    for i in 0..count {
        let child = ffi::CFArrayGetValueAtIndex(children, i) as ffi::AXUIElementRef;
        if let Ok(button) = collect_buttons_recursive(child, target_name, false) {
            ffi::CFRelease(children);
            return Ok(button);
        }
    }

    ffi::CFRelease(children);
    bail!("Button '{}' not found", target_name)
}

/// Recursively collect all button names
unsafe fn collect_buttons_list_recursive(
    element: ffi::AXUIElementRef,
    buttons: &mut Vec<String>,
) -> Result<()> {
    // Check if this element is a button
    if let Ok(Some(role)) = ffi::get_ax_string_attribute(element, "AXRole") {
        if role == "AXButton" {
            if let Ok(Some(title)) = ffi::get_ax_string_attribute(element, "AXTitle") {
                if !title.is_empty() {
                    buttons.push(title);
                }
            }
        }
    }

    // Get children
    let children_attr = ffi::create_cfstring("AXChildren");
    let mut children: *mut c_void = ptr::null_mut();
    let result = ffi::AXUIElementCopyAttributeValue(element, children_attr, &mut children);
    ffi::CFRelease(children_attr);

    if result == ffi::K_AX_ERROR_SUCCESS && !children.is_null() {
        let count = ffi::CFArrayGetCount(children);
        for i in 0..count {
            let child = ffi::CFArrayGetValueAtIndex(children, i) as ffi::AXUIElementRef;
            let _ = collect_buttons_list_recursive(child, buttons);
        }
        ffi::CFRelease(children);
    }

    Ok(())
}

/// Get all static text in a window
unsafe fn get_all_text_in_window(window: ffi::AXUIElementRef) -> Result<Vec<String>> {
    let mut text_elements = Vec::new();
    collect_text_recursive(window, &mut text_elements)?;
    Ok(text_elements)
}

/// Recursively collect all static text
unsafe fn collect_text_recursive(
    element: ffi::AXUIElementRef,
    text_elements: &mut Vec<String>,
) -> Result<()> {
    // Check if this element is static text
    if let Ok(Some(role)) = ffi::get_ax_string_attribute(element, "AXRole") {
        if role == "AXStaticText" {
            if let Ok(Some(value)) = ffi::get_ax_string_attribute(element, "AXValue") {
                if !value.is_empty() {
                    text_elements.push(value);
                }
            }
        }
    }

    // Get children
    let children_attr = ffi::create_cfstring("AXChildren");
    let mut children: *mut c_void = ptr::null_mut();
    let result = ffi::AXUIElementCopyAttributeValue(element, children_attr, &mut children);
    ffi::CFRelease(children_attr);

    if result == ffi::K_AX_ERROR_SUCCESS && !children.is_null() {
        let count = ffi::CFArrayGetCount(children);
        for i in 0..count {
            let child = ffi::CFArrayGetValueAtIndex(children, i) as ffi::AXUIElementRef;
            let _ = collect_text_recursive(child, text_elements);
        }
        ffi::CFRelease(children);
    }

    Ok(())
}
