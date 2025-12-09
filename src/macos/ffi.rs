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
// Core Graphics Event Framework (for keystroke sending)
// ============================================================================

pub const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
pub const CG_HID_EVENT_TAP: u32 = 0;

// Modifier key flags
pub const CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
pub const CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
pub const CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000; // Option key
pub const CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;

    pub fn CGEventSetFlags(event: *mut c_void, flags: u64);
    pub fn CGEventPost(tap: u32, event: *mut c_void);
    pub fn CGEventSourceCreate(source_state_id: i32) -> *mut c_void;
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

/// Get accessibility string attribute safely
///
/// Returns Ok(None) if attribute doesn't exist or isn't a string
/// Returns Err if accessibility API fails
pub unsafe fn get_ax_string_attribute(
    element: AXUIElementRef,
    attribute: &str,
) -> anyhow::Result<Option<String>> {
    let attr_key = create_cfstring(attribute);
    let mut value: *mut c_void = std::ptr::null_mut();
    let result = AXUIElementCopyAttributeValue(element, attr_key, &mut value);
    CFRelease(attr_key);

    if result != K_AX_ERROR_SUCCESS {
        return Ok(None);
    }

    let text = if !value.is_null() {
        let s = cfstring_to_string(value);
        CFRelease(value);
        s
    } else {
        None
    };
    Ok(text)
}

/// Create app accessibility element safely
pub unsafe fn create_app_element(pid: i32) -> anyhow::Result<AXUIElementRef> {
    let element = AXUIElementCreateApplication(pid);
    if element.is_null() {
        anyhow::bail!("Failed to create accessibility element for PID {}", pid)
    }
    Ok(element)
}

/// Get PID for app by name (authoritative version)
///
/// Uses soft_match for fuzzy name matching
pub fn get_pid_by_name(app_name: &str) -> anyhow::Result<i32> {
    use objc2::{class, msg_send};
    use objc2::runtime::AnyObject;

    unsafe {
        let workspace_class = class!(NSWorkspace);
        let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
        let running_apps: *mut AnyObject = msg_send![workspace, runningApplications];
        let count: usize = msg_send![running_apps, count];

        for i in 0..count {
            let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
            let name_ns: *mut c_void = msg_send![app, localizedName];

            if !name_ns.is_null() {
                if let Some(name) = cfstring_to_string(name_ns) {
                    if crate::soft_match(&name, app_name) {
                        let pid: i32 = msg_send![app, processIdentifier];
                        return Ok(pid);
                    }
                }
            }
        }

        anyhow::bail!("App '{}' not found in running applications", app_name)
    }
}
