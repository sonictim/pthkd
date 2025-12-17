//! Helper functions and RAII wrappers for macOS FFI
//!
//! This module centralizes common patterns to reduce code duplication and
//! improve memory safety through RAII wrappers.

use super::ffi::*;
use anyhow::{Result, bail};
use std::ffi::c_void;
use std::ptr;

// ============================================================================
// RAII Wrappers for Automatic Resource Cleanup
// ============================================================================

/// RAII wrapper for CFStringRef that automatically releases on drop
pub struct CFString(pub *mut c_void);

impl CFString {
    /// Create a CFString from a Rust str
    pub unsafe fn new(s: &str) -> Self {
        unsafe { Self(create_cfstring(s)) }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }

    /// Check if the pointer is null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for CFString {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

/// RAII wrapper for AXUIElementRef that automatically releases on drop
pub struct AXElement(pub AXUIElementRef);

impl AXElement {
    /// Wrap an existing AXUIElementRef
    pub unsafe fn new(element: AXUIElementRef) -> Self {
        Self(element)
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> AXUIElementRef {
        self.0
    }

    /// Check if the pointer is null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for AXElement {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

// ============================================================================
// NSWorkspace / Application Helpers
// ============================================================================

/// Execute a closure with access to a running application
///
/// Eliminates the duplicated NSWorkspace lookup pattern
pub unsafe fn with_running_app<F, R>(app_name: &str, f: F) -> Result<R>
where
    F: FnOnce(*mut objc2::runtime::AnyObject) -> Result<R>,
{
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};

    let workspace_class = AnyClass::get("NSWorkspace")
        .ok_or_else(|| anyhow::anyhow!("Failed to get NSWorkspace class"))?;

    let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
    if workspace.is_null() {
        bail!("Failed to get NSWorkspace");
    }

    let running_apps: *mut AnyObject = msg_send![workspace, runningApplications];
    if running_apps.is_null() {
        bail!("Failed to get running applications");
    }

    let count: usize = msg_send![running_apps, count];

    for i in 0..count {
        let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
        if app.is_null() {
            continue;
        }

        let localized_name: *mut AnyObject = msg_send![app, localizedName];
        if localized_name.is_null() {
            continue;
        }

        if let Some(name) = unsafe { cfstring_to_string(localized_name as *mut c_void) }
            && crate::soft_match(&name, app_name)
        {
            return f(app);
        }
    }

    bail!("No running application found matching '{}'", app_name)
}

// ============================================================================
// Accessibility API Helpers
// ============================================================================

/// Get a string attribute from an AXUIElement
///
/// Handles CFString creation, attribute lookup, conversion, and cleanup
pub unsafe fn get_ax_string_attr(element: AXUIElementRef, attr_name: &str) -> Result<String> {
    unsafe {
        let attr = CFString::new(attr_name);
        let mut value: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(element, attr.as_ptr(), &mut value);

        if result != K_AX_ERROR_SUCCESS || value.is_null() {
            bail!("Failed to get attribute '{}'", attr_name);
        }

        let text = cfstring_to_string(value).ok_or_else(|| {
            anyhow::anyhow!("Failed to convert attribute '{}' to string", attr_name)
        })?;

        CFRelease(value);

        Ok(text)
    }
}

/// Get an element attribute from an AXUIElement
///
/// Returns the element wrapped in AXElement for automatic cleanup
pub unsafe fn get_ax_element_attr(element: AXUIElementRef, attr_name: &str) -> Result<AXElement> {
    unsafe {
        let attr = CFString::new(attr_name);
        let mut value: *mut c_void = ptr::null_mut();

        let result = AXUIElementCopyAttributeValue(element, attr.as_ptr(), &mut value);

        if result != K_AX_ERROR_SUCCESS || value.is_null() {
            bail!("Failed to get element attribute '{}'", attr_name);
        }

        Ok(AXElement::new(value as AXUIElementRef))
    }
}

/// Execute a closure with access to an application's window
///
/// Handles app element creation, window lookup, and cleanup
pub unsafe fn with_app_window<F, R>(app_name: &str, window_name: &str, f: F) -> Result<R>
where
    F: FnOnce(AXUIElementRef, AXUIElementRef) -> Result<R>,
{
    unsafe {
        let pid = super::app_info::get_pid_by_name(app_name)?;
        let app_element = AXElement::new(AXUIElementCreateApplication(pid));

        let window = if window_name.is_empty() {
            super::ui_elements::get_focused_window(app_element.as_ptr())?
        } else {
            super::ui_elements::find_window_by_name(app_element.as_ptr(), window_name)?
        };

        // Note: window is a retained reference (from AXUIElementCopyAttributeValue),
        // so we need to release it after use
        let result = f(app_element.as_ptr(), window);

        CFRelease(window);

        result
    }
}
