//! Helper functions and RAII wrappers for macOS FFI
//!
//! This module centralizes common patterns to reduce code duplication and
//! improve memory safety through RAII wrappers.

use super::ffi::*;
use anyhow::{Result, bail};
use std::ffi::c_void;

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

/// RAII wrapper for CGEvent that automatically releases on drop
pub struct CGEvent(pub *mut c_void);

impl CGEvent {
    /// Wrap an existing CGEvent pointer
    pub unsafe fn new(event: *mut c_void) -> Self {
        Self(event)
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

impl Drop for CGEvent {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

/// RAII wrapper for CFArray that automatically releases on drop
pub struct CFArray(pub *mut c_void);

impl CFArray {
    /// Wrap an existing CFArray pointer
    pub unsafe fn new(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }

    /// Get array count
    pub fn count(&self) -> isize {
        unsafe { super::ffi::CFArrayGetCount(self.0) }
    }

    /// Get element at index (returns borrowed reference)
    pub fn get(&self, index: isize) -> *mut c_void {
        unsafe { super::ffi::CFArrayGetValueAtIndex(self.0, index) }
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for CFArray {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                super::ffi::CFRelease(self.0);
            }
        }
    }
}

/// RAII wrapper for CFNumber that automatically releases on drop
pub struct CFNumber(pub *mut c_void);

impl CFNumber {
    /// Create CFNumber from i32
    pub unsafe fn from_i32(value: i32) -> Self {
        Self(unsafe {
            super::ffi::CFNumberCreate(
                std::ptr::null(),
                9, // kCFNumberSInt32Type
                &value as *const i32 as *const libc::c_void,
            )
        })
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for CFNumber {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                super::ffi::CFRelease(self.0);
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
    use objc2::runtime::AnyObject;
    use super::session::MacOSSession;

    // Use building block to get running apps
    let os = MacOSSession::global();
    let running_apps = os.get_running_apps()?;

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
