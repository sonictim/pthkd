//! Central macOS Cocoa session for UI operations
//!
//! This module provides a cohesive API for macOS Cocoa operations with proper lifecycle management.
//! Similar to ProtoolsSession, MacOSSession consolidates all duplicated NSString/NSAlert/Objective-C
//! patterns into a single, reusable session object.
//!
//! Different modules can extend MacOSSession via `impl` blocks for their specific needs.

use anyhow::{Context, Result};
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, Encode, Encoding};
use std::sync::OnceLock;

// ============================================================================
// Geometry Types (NSRect, NSPoint, NSSize)
// ============================================================================

/// CoreGraphics point (x, y coordinates)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NSPoint {
    pub x: f64,
    pub y: f64,
}

/// CoreGraphics size (width, height)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NSSize {
    pub width: f64,
    pub height: f64,
}

/// CoreGraphics rectangle (origin + size)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NSRect {
    pub origin: NSPoint,
    pub size: NSSize,
}

// Implement Encode for objc2 message passing
unsafe impl Encode for NSPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}

unsafe impl Encode for NSSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}

unsafe impl Encode for NSRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[NSPoint::ENCODING, NSSize::ENCODING]);
}

// ============================================================================
// MacOSSession - Core Session Structure
// ============================================================================

/// Central macOS Cocoa session for UI operations
///
/// Similar to ProtoolsSession, this provides a cohesive API for macOS Cocoa operations
/// with proper lifecycle management. Access via `MacOSSession::global()`.
///
/// # Example
/// ```ignore
/// let os = MacOSSession::global();
/// os.show_alert("Title", "Message", &["OK"])?;
/// os.create_nsstring("Hello")?;
/// ```
pub struct MacOSSession {
    /// Cache of commonly-used Objective-C classes
    class_cache: ClassCache,
}

/// Cache of commonly-used Objective-C classes to avoid repeated lookups
struct ClassCache {
    ns_string: &'static AnyClass,
    ns_alert: &'static AnyClass,
    ns_window: &'static AnyClass,
    ns_menu: &'static AnyClass,
    ns_menu_item: &'static AnyClass,
    ns_image: &'static AnyClass,
}

/// Global macOS session singleton
static MACOS_SESSION: OnceLock<MacOSSession> = OnceLock::new();

// ============================================================================
// Core Session Implementation
// ============================================================================

impl MacOSSession {
    /// Get the global macOS session (initialized on first use)
    ///
    /// The session is created once and reused for all operations.
    /// This matches the TOKIO_RT pattern used elsewhere in the codebase.
    pub fn global() -> &'static MacOSSession {
        MACOS_SESSION.get_or_init(|| unsafe {
            MacOSSession::new().expect("Failed to initialize macOS session")
        })
    }

    /// Create a new macOS session
    ///
    /// # Safety
    /// This function calls Objective-C runtime functions
    unsafe fn new() -> Result<Self> {
        Ok(Self {
            class_cache: unsafe { ClassCache::new()? },
        })
    }

    /// Get a cached or lookup Objective-C class
    ///
    /// # Example
    /// ```ignore
    /// let scroll_view_class = os.get_class("NSScrollView")?;
    /// ```
    pub fn get_class(&self, name: &str) -> Result<&'static AnyClass> {
        AnyClass::get(name).context(format!("Failed to get {} class", name))
    }

    /// Generic alloc + init pattern for Objective-C objects
    ///
    /// This consolidates the common pattern of allocating and initializing
    /// an Objective-C object, which appears 22+ times across the codebase.
    ///
    /// # Safety
    /// Calls Objective-C runtime functions
    ///
    /// # Example
    /// ```ignore
    /// let scroll_view_class = os.get_class("NSScrollView")?;
    /// let scroll_view = os.alloc_init(scroll_view_class)?;
    /// ```
    pub unsafe fn alloc_init(&self, class: &AnyClass) -> Result<*mut AnyObject> {
        let obj: *mut AnyObject = msg_send![class, alloc];
        let obj: *mut AnyObject = msg_send![obj, init];
        if obj.is_null() {
            anyhow::bail!("Failed to alloc+init object");
        }
        Ok(obj)
    }

    /// Create NSString from Rust &str
    ///
    /// This consolidates the NSString creation pattern that appears 4+ times
    /// across the codebase (window.rs, keyring.rs, notifications.rs, permissions.rs).
    ///
    /// # Safety
    /// Calls Objective-C runtime functions
    ///
    /// # Example
    /// ```ignore
    /// let title = os.create_nsstring("Hello World")?;
    /// let _: () = msg_send![window, setTitle: title];
    /// ```
    pub unsafe fn create_nsstring(&self, text: &str) -> Result<*mut AnyObject> {
        let ns_string: *mut AnyObject = msg_send![self.class_cache.ns_string, alloc];
        let ns_string: *mut AnyObject = msg_send![
            ns_string,
            initWithBytes: text.as_ptr() as *const std::ffi::c_void
            length: text.len()
            encoding: 4_usize  // NSUTF8StringEncoding = 4
        ];

        if ns_string.is_null() {
            anyhow::bail!("Failed to create NSString");
        }

        Ok(ns_string)
    }

    // ========================================================================
    // Alert Building Blocks (Composable - Used Across Multiple Functions)
    // ========================================================================

    /// Create a new NSAlert (empty, ready to configure)
    ///
    /// **CROSSOVER:** Used by show_alert, password_prompt, show_permission_dialog
    pub unsafe fn create_alert(&self) -> Result<*mut AnyObject> {
        self.alloc_init(self.class_cache.ns_alert)
    }

    /// Set alert style (informational=1, warning=0, critical=2)
    ///
    /// **CROSSOVER:** Used by show_permission_dialog (critical alerts)
    pub unsafe fn set_alert_style(&self, alert: *mut AnyObject, style: i64) -> Result<()> {
        let _: () = msg_send![alert, setAlertStyle: style];
        Ok(())
    }

    /// Set alert title and message text
    ///
    /// **CROSSOVER:** Used by ALL alert functions (show_alert, password_prompt, permission_dialog)
    pub unsafe fn set_alert_text(&self, alert: *mut AnyObject, title: &str, message: &str) -> Result<()> {
        let title_str = self.create_nsstring(title)?;
        let msg_str = self.create_nsstring(message)?;
        let _: () = msg_send![alert, setMessageText: title_str];
        let _: () = msg_send![alert, setInformativeText: msg_str];
        Ok(())
    }

    /// Add a button to an alert
    ///
    /// Buttons added in order: first=1000 (default), second=1001, third=1002, etc.
    ///
    /// **CROSSOVER:** Used by ALL alert functions
    pub unsafe fn add_alert_button(&self, alert: *mut AnyObject, title: &str) -> Result<()> {
        let btn_str = self.create_nsstring(title)?;
        let _: () = msg_send![alert, addButtonWithTitle: btn_str];
        Ok(())
    }

    /// Add an accessory view to an alert (for custom UI like text fields)
    ///
    /// **CROSSOVER:** Used by password_prompt (adds NSSecureTextField)
    pub unsafe fn add_accessory_view(&self, alert: *mut AnyObject, view: *mut AnyObject) -> Result<()> {
        let _: () = msg_send![alert, setAccessoryView: view];
        Ok(())
    }

    /// Show modal alert and return button response
    ///
    /// Returns: 1000=first button, 1001=second, 1002=third, etc.
    ///
    /// **CROSSOVER:** Used by ALL alert functions
    pub unsafe fn show_modal_alert(&self, alert: *mut AnyObject) -> Result<isize> {
        let response: isize = msg_send![alert, runModal];
        Ok(response)
    }

    /// Show a simple alert dialog (convenience method built from building blocks)
    ///
    /// # Example
    /// ```ignore
    /// let response = os.show_alert("Error", "Something went wrong", &["OK", "Cancel"])?;
    /// if response == 1000 { /* User clicked OK */ }
    /// ```
    pub unsafe fn show_alert(&self, title: &str, message: &str, buttons: &[&str]) -> Result<isize> {
        let alert = self.create_alert()?;
        self.set_alert_text(alert, title, message)?;
        for button in buttons {
            self.add_alert_button(alert, button)?;
        }
        self.show_modal_alert(alert)
    }

    /// Helper to create NSRect from coordinates
    ///
    /// # Example
    /// ```ignore
    /// let frame = os.rect(100.0, 100.0, 400.0, 300.0);
    /// ```
    pub fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
        NSRect {
            origin: NSPoint { x, y },
            size: NSSize { width, height },
        }
    }
}

// ============================================================================
// ClassCache Implementation
// ============================================================================

impl ClassCache {
    /// Initialize the class cache with commonly-used Objective-C classes
    unsafe fn new() -> Result<Self> {
        Ok(Self {
            ns_string: AnyClass::get("NSString")
                .context("Failed to get NSString class")?,
            ns_alert: AnyClass::get("NSAlert")
                .context("Failed to get NSAlert class")?,
            ns_window: AnyClass::get("NSWindow")
                .context("Failed to get NSWindow class")?,
            ns_menu: AnyClass::get("NSMenu")
                .context("Failed to get NSMenu class")?,
            ns_menu_item: AnyClass::get("NSMenuItem")
                .context("Failed to get NSMenuItem class")?,
            ns_image: AnyClass::get("NSImage")
                .context("Failed to get NSImage class")?,
        })
    }
}

// ============================================================================
// Thread Safety
// ============================================================================

unsafe impl Send for MacOSSession {}
unsafe impl Sync for MacOSSession {}
