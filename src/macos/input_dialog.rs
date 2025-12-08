//! Input dialog functionality via NSAlert + NSTextField
//!
//! STATUS: EXPERIMENTAL - Work in progress
//!
//! Provides a simple modal dialog with a text input field.
//! Uses NSAlert with an NSTextField accessory view for native macOS appearance.

use super::ffi::*;
use anyhow::Result;
use libc::c_void;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create an NSString from a Rust &str
///
/// NSString and CFString are toll-free bridged, so we can use CFString functions
unsafe fn create_nsstring(s: &str) -> *mut c_void {
    create_cfstring(s)
}

/// Convert an NSString to a Rust String
///
/// NSString and CFString are toll-free bridged, so we can use CFString functions
unsafe fn nsstring_to_string(ns: *mut c_void) -> String {
    cfstring_to_string(ns).unwrap_or_default()
}

// ============================================================================
// NSRect Structure
// ============================================================================

/// NSRect structure for creating NSTextField frame
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct NSRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

// Implement Encode trait for NSRect so it can be passed through msg_send!
unsafe impl objc2::encode::Encode for NSRect {
    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct(
        "CGRect",
        &[
            objc2::encode::Encoding::Double,
            objc2::encode::Encoding::Double,
            objc2::encode::Encoding::Double,
            objc2::encode::Encoding::Double,
        ],
    );
}

// Also implement RefEncode for references
unsafe impl objc2::encode::RefEncode for NSRect {
    const ENCODING_REF: objc2::encode::Encoding = objc2::encode::Encoding::Pointer(&<NSRect as objc2::encode::Encode>::ENCODING);
}

// ============================================================================
// Modal Response Constants
// ============================================================================

/// NSModalResponse constants for button clicks
const NS_MODAL_RESPONSE_OK: isize = 1000;
const NS_MODAL_RESPONSE_CANCEL: isize = 1001;

// ============================================================================
// Internal Implementation
// ============================================================================

/// Internal implementation of input dialog
///
/// Creates NSAlert with NSTextField accessory view and shows it modally
unsafe fn show_input_dialog_internal(
    title: &str,
    prompt: Option<&str>,
    default_value: Option<&str>,
) -> Result<Option<String>> {
    // Create an autorelease pool for Objective-C memory management
    let pool_class = class!(NSAutoreleasePool);
    let pool: *mut AnyObject = msg_send![pool_class, new];

    // Create NSAlert: [[NSAlert alloc] init]
    let alert_class = class!(NSAlert);
    let alert: *mut AnyObject = msg_send![alert_class, alloc];
    let alert: *mut AnyObject = msg_send![alert, init];

    if alert.is_null() {
        anyhow::bail!("Failed to create NSAlert");
    }

    // Transform app from background daemon to foreground app temporarily
    // This is required for the app to receive keyboard input
    // Get NSApp (shared application instance)
    let ns_app_class = class!(NSApplication);
    let ns_app: *mut AnyObject = msg_send![ns_app_class, sharedApplication];

    // Save current activation policy
    let current_policy: isize = msg_send![ns_app, activationPolicy];
    log::info!("Current activation policy: {}", current_policy);

    // NSApplicationActivationPolicyRegular = 0 (normal app, appears in Dock)
    // NSApplicationActivationPolicyAccessory = 1 (menu bar app)
    // NSApplicationActivationPolicyProhibited = 2 (background only, no UI)
    let regular_policy: isize = 0;

    // Change to regular app so we can receive keyboard input
    let _: bool = msg_send![ns_app, setActivationPolicy: regular_policy];
    log::info!("Changed to regular activation policy for dialog");

    // Activate ignoring other apps (brings to front)
    let _: () = msg_send![ns_app, activateIgnoringOtherApps: true];

    // Set alert style to critical
    // NSAlertStyleCritical = 2
    let _: () = msg_send![alert, setAlertStyle: 2_isize];

    // Set title (messageText): alert.messageText = @"Title"
    let title_ns = create_nsstring(title);
    let _: () = msg_send![alert, setMessageText: title_ns];

    // Set prompt (informativeText) if provided
    if let Some(prompt_text) = prompt {
        let prompt_ns = create_nsstring(prompt_text);
        let _: () = msg_send![alert, setInformativeText: prompt_ns];
    }

    // Create NSTextField (300x24 pixels)
    // [[NSTextField alloc] initWithFrame:NSMakeRect(0, 0, 300, 24)]
    let textfield_class = class!(NSTextField);
    let frame = NSRect {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 24.0,
    };
    let textfield: *mut AnyObject = msg_send![textfield_class, alloc];
    let textfield: *mut AnyObject = msg_send![textfield, initWithFrame: frame];

    if textfield.is_null() {
        anyhow::bail!("Failed to create NSTextField");
    }

    // Set default value if provided: input.stringValue = @"default"
    if let Some(default) = default_value {
        let default_ns = create_nsstring(default);
        let _: () = msg_send![textfield, setStringValue: default_ns];
    }

    // Set text field as accessory view: alert.accessoryView = input
    let _: () = msg_send![alert, setAccessoryView: textfield];

    // Add OK button: [alert addButtonWithTitle:@"OK"]
    let ok_ns = create_nsstring("OK");
    let _: *mut AnyObject = msg_send![alert, addButtonWithTitle: ok_ns];

    // Add Cancel button: [alert addButtonWithTitle:@"Cancel"]
    let cancel_ns = create_nsstring("Cancel");
    let _: *mut AnyObject = msg_send![alert, addButtonWithTitle: cancel_ns];

    // Get the alert's window and configure it to float on top
    // We need to show the alert first to get its window
    let window: *mut AnyObject = msg_send![alert, window];
    if !window.is_null() {
        // NSWindowLevel constants:
        // NSFloatingWindowLevel = 3
        // NSStatusWindowLevel = 25
        // NSModalPanelWindowLevel = 8
        let floating_level: isize = 25; // NSStatusWindowLevel - above everything
        let _: () = msg_send![window, setLevel: floating_level];

        // Make it key and front
        let _: () = msg_send![window, makeKeyAndOrderFront: std::ptr::null_mut::<c_void>()];

        // Make text field first responder (give it focus)
        let _: bool = msg_send![window, makeFirstResponder: textfield];
    }

    // Show modal dialog (blocks until user responds): [alert runModal]
    let response: isize = msg_send![alert, runModal];

    // Check response and prepare result
    let result = if response == NS_MODAL_RESPONSE_OK {
        // Get text from text field: NSString *text = input.stringValue
        let result_ns: *mut c_void = msg_send![textfield, stringValue];

        // Convert to Rust String while pool is still active
        let text = if result_ns.is_null() {
            String::new()
        } else {
            nsstring_to_string(result_ns)
        };

        Some(text)
    } else {
        // User cancelled (Cancel button or ESC key)
        None
    };

    // Restore previous activation policy
    let _: bool = msg_send![ns_app, setActivationPolicy: current_policy];
    log::info!("Restored activation policy to {}", current_policy);

    // Drain the autorelease pool
    let _: () = msg_send![pool, drain];

    Ok(result)
}

// ============================================================================
// Public API
// ============================================================================

/// Show a modal input dialog and return the user's text input
///
/// This function displays a native macOS alert dialog with a text input field.
/// The dialog is modal and blocks until the user responds.
///
/// **Note:** This must be called from the main thread. The event tap callback
/// typically runs on the main thread, so this should work when called from
/// hotkey actions.
///
/// # Parameters
/// * `title` - The main message text (prominent text at top)
/// * `prompt` - Optional informative text (smaller text below title)
/// * `default_value` - Optional pre-filled text in the input field
///
/// # Returns
/// * `Ok(Some(String))` - User clicked OK/Submit with text (even if empty)
/// * `Ok(None)` - User clicked Cancel or closed dialog
/// * `Err(_)` - System error (permissions, API failure, etc.)
///
/// # Example
/// ```ignore
/// match show_input_dialog("Enter track name:", None, Some("Track 1"))? {
///     Some(text) => println!("User entered: {}", text),
///     None => println!("User cancelled"),
/// }
/// ```
pub fn show_input_dialog(
    title: &str,
    prompt: Option<&str>,
    default_value: Option<&str>,
) -> Result<Option<String>> {
    log::info!("Showing input dialog: '{}'", title);

    // Call directly - NSAlert must run on the main thread
    // The event tap callback runs on the main thread, so this should work
    let result = unsafe { show_input_dialog_internal(title, prompt, default_value) };

    match &result {
        Ok(Some(text)) => log::info!("User entered: '{}'", text),
        Ok(None) => log::info!("User cancelled dialog"),
        Err(e) => log::error!("Dialog error: {}", e),
    }

    result
}
