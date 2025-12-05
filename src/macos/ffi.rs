//! Shared FFI declarations for macOS system frameworks
//!
//! This module provides a single source of truth for FFI bindings used across
//! multiple macOS integration modules.

use libc::c_void;

// ============================================================================
// Core Foundation Framework
// ============================================================================

pub const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

pub type CFStringRef = *mut c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Release a Core Foundation object
    pub fn CFRelease(cf: *mut c_void);

    /// Create a CFString from a C string
    pub fn CFStringCreateWithCString(
        alloc: *mut c_void,
        c_str: *const i8,
        encoding: u32,
    ) -> *mut c_void;

    /// Copy CFString contents to a C string buffer
    pub fn CFStringGetCString(
        the_string: *mut c_void,
        buffer: *mut u8,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;

    /// Get the length of a CFString
    pub fn CFStringGetLength(the_string: *mut c_void) -> isize;

    /// Get the count of elements in a CFArray
    pub fn CFArrayGetCount(the_array: *mut c_void) -> isize;

    /// Get an element at an index in a CFArray
    pub fn CFArrayGetValueAtIndex(the_array: *mut c_void, idx: isize) -> *mut c_void;

    /// Get the type ID of a Core Foundation object
    pub fn CFGetTypeID(cf: *mut c_void) -> usize;

    /// Get the type ID for CFString type
    pub fn CFStringGetTypeID() -> usize;
}

// ============================================================================
// Accessibility Framework
// ============================================================================

/// Error codes from Accessibility API
pub const K_AX_ERROR_SUCCESS: i32 = 0;
pub const K_AX_ERROR_INVALID_UI_ELEMENT: i32 = -25204;
pub const K_AX_ERROR_API_DISABLED: i32 = -25211;
pub const K_AX_ERROR_NO_VALUE: i32 = -25212;

pub type AXUIElementRef = *mut c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// Create an accessibility element for an application by PID
    pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;

    /// Create a system-wide accessibility element
    pub fn AXUIElementCreateSystemWide() -> AXUIElementRef;

    /// Copy the value of an accessibility attribute
    pub fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut *mut c_void,
    ) -> i32;

    /// Perform an accessibility action
    pub fn AXUIElementPerformAction(
        element: AXUIElementRef,
        action: CFStringRef,
    ) -> i32;

    /// Check if the current process is trusted for accessibility
    pub fn AXIsProcessTrusted() -> bool;
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to create a CFString from a Rust &str
pub unsafe fn create_cfstring(s: &str) -> *mut c_void {
    let c_str = std::ffi::CString::new(s).unwrap();
    CFStringCreateWithCString(std::ptr::null_mut(), c_str.as_ptr(), K_CF_STRING_ENCODING_UTF8)
}

/// Check if a Core Foundation object is a CFString
pub unsafe fn is_cfstring(value: *mut c_void) -> bool {
    if value.is_null() {
        return false;
    }
    CFGetTypeID(value) == CFStringGetTypeID()
}

/// Helper to convert CFString to Rust String
pub unsafe fn cfstring_to_string(cfstring: *mut c_void) -> Option<String> {
    if cfstring.is_null() {
        return None;
    }

    // Check if it's actually a CFString before calling CFString functions
    if !is_cfstring(cfstring) {
        return None;
    }

    let length = CFStringGetLength(cfstring);
    if length == 0 {
        return Some(String::new());
    }

    // Allocate buffer with extra space for null terminator
    let buffer_size = (length * 4 + 1) as usize; // UTF-8 can be up to 4 bytes per char
    let mut buffer = vec![0u8; buffer_size];

    let success = CFStringGetCString(
        cfstring,
        buffer.as_mut_ptr(),
        buffer_size as isize,
        K_CF_STRING_ENCODING_UTF8,
    );

    if success {
        // Find the null terminator and create string from bytes
        let null_pos = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        String::from_utf8(buffer[..null_pos].to_vec()).ok()
    } else {
        None
    }
}
