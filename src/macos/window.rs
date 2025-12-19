//! Native macOS window for displaying text
//!
//! This module provides a simple text display window using native Objective-C
//! via objc2. It creates an NSWindow with NSTextView to show text content.
//!
//! STATUS: EXPERIMENTAL - Learning example for Objective-C patterns

use objc2::runtime::{AnyClass, AnyObject};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use objc2::{Encode, Encoding, msg_send};
use std::ptr;

// NSRect, NSPoint, and NSSize definitions for Cocoa
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NSPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NSSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NSRect {
    origin: NSPoint,
    size: NSSize,
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

/// Creates an NSString from a Rust &str
///
/// # Safety
/// Uses FFI to call Objective-C methods
unsafe fn create_nsstring(text: &str) -> Result<*mut objc2::runtime::AnyObject> {
    let class = AnyClass::get("NSString").context("Failed to get NSString class")?;
    let ns_string: *mut objc2::runtime::AnyObject = msg_send![class, alloc];
    let ns_string: *mut objc2::runtime::AnyObject = msg_send![
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

/// Shows a simple window displaying the provided text
///
/// Creates a native macOS window (NSWindow) with an NSTextView containing
/// the provided text. The window is non-editable but text is selectable.
///
/// # Parameters
/// * `text` - The text to display in the window
///
/// # Returns
/// * `Ok(())` - Window was created and displayed successfully
/// * `Err(_)` - Failed to create window or set up UI elements
///
/// # Example
/// ```ignore
/// show_text_window("Hello World!")?;
/// ```
pub fn show_text_window(text: &str) -> Result<()> {
    unsafe {
        // 1. Get NSWindow class and create window
        let window_class = AnyClass::get("NSWindow").context("Failed to get NSWindow class")?;
        let window: *mut objc2::runtime::AnyObject = msg_send![window_class, alloc];

        // Create window with frame (x=100, y=100, width=400, height=300)
        // initWithContentRect:styleMask:backing:defer:
        //   - contentRect: NSRect (origin, size)
        //   - styleMask: 15 = Titled + Closable + Miniaturizable + Resizable
        //   - backing: 2 = NSBackingStoreBuffered
        //   - defer: false (create immediately)
        let frame = NSRect {
            origin: NSPoint { x: 100.0, y: 100.0 },
            size: NSSize {
                width: 800.0,
                height: 600.0,
            },
        };
        let window: *mut objc2::runtime::AnyObject = msg_send![
            window,
            initWithContentRect: frame
            styleMask: 15_usize
            backing: 2_usize
            defer: false
        ];

        if window.is_null() {
            anyhow::bail!("Failed to create NSWindow");
        }

        // 2. Set window title and prevent deallocation
        let title = create_nsstring("Output")?;
        let _: () = msg_send![window, setTitle: title];

        // Keep window alive (don't release when closed)
        let _: () = msg_send![window, setReleasedWhenClosed: false];

        // 3. Create NSScrollView to hold the text view
        let scroll_view_class =
            AnyClass::get("NSScrollView").context("Failed to get NSScrollView class")?;
        let scroll_view: *mut objc2::runtime::AnyObject = msg_send![scroll_view_class, alloc];
        let scroll_view_frame = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width: 800.0,
                height: 600.0,
            },
        };
        let scroll_view: *mut objc2::runtime::AnyObject = msg_send![
            scroll_view,
            initWithFrame: scroll_view_frame
        ];

        if scroll_view.is_null() {
            anyhow::bail!("Failed to create NSScrollView");
        }

        // Configure scroll view
        let _: () = msg_send![scroll_view, setHasVerticalScroller: true];
        let _: () = msg_send![scroll_view, setHasHorizontalScroller: true];
        let _: () = msg_send![scroll_view, setAutohidesScrollers: true];
        let _: () = msg_send![scroll_view, setBorderType: 0_usize]; // NSNoBorder = 0

        // 4. Create NSTextView for displaying text
        let text_view_class =
            AnyClass::get("NSTextView").context("Failed to get NSTextView class")?;
        let text_view: *mut objc2::runtime::AnyObject = msg_send![text_view_class, alloc];

        // Get the content size from the scroll view (accounts for scrollers)
        let content_size: NSSize = msg_send![scroll_view, contentSize];
        let text_view_frame = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: content_size,
        };
        let text_view: *mut objc2::runtime::AnyObject = msg_send![
            text_view,
            initWithFrame: text_view_frame
        ];

        if text_view.is_null() {
            anyhow::bail!("Failed to create NSTextView");
        }

        // 5. Configure text view properties
        let _: () = msg_send![text_view, setEditable: false]; // Read-only
        let _: () = msg_send![text_view, setSelectable: true]; // Allow text selection

        // Enable text wrapping
        let _: () = msg_send![text_view, setHorizontallyResizable: false];
        let _: () = msg_send![text_view, setVerticallyResizable: true];

        // Get the text container and set its width to match scroll view
        let text_container: *mut AnyObject = msg_send![text_view, textContainer];
        let _: () = msg_send![text_container, setContainerSize: NSSize { width: content_size.width, height: 10000000.0 }];
        let _: () = msg_send![text_container, setWidthTracksTextView: true];

        // 6. Set text content
        let content = create_nsstring(text)?;
        let _: () = msg_send![text_view, setString: content];

        // 7. Add text view as document view of scroll view
        let _: () = msg_send![scroll_view, setDocumentView: text_view];

        // 8. Add scroll view to window's content view
        let content_view: *mut objc2::runtime::AnyObject = msg_send![window, contentView];
        let _: () = msg_send![content_view, addSubview: scroll_view];

        // 9. Show window and make it active
        let _: () =
            msg_send![window, makeKeyAndOrderFront: ptr::null::<objc2::runtime::AnyObject>()];

        log::info!("Text window displayed successfully");

        Ok(())
    }
}

/// Show an About dialog with version information
pub unsafe fn show_message_dialog(message: &str) {
    use objc2::{msg_send, runtime::AnyClass, sel};

    // Get NSAlert class
    let alert_class = match AnyClass::get("NSAlert") {
        Some(c) => c,
        None => {
            log::error!("Failed to get NSAlert class");
            return;
        }
    };

    // Create alert
    let alert: *mut AnyObject = msg_send![alert_class, alloc];
    let alert: *mut AnyObject = msg_send![alert, init];

    // Set message text
    let ns_string_class = match AnyClass::get("NSString") {
        Some(c) => c,
        None => {
            log::error!("Failed to get NSString class");
            return;
        }
    };

    let message_string: *mut AnyObject = msg_send![ns_string_class, alloc];
    let message_string: *mut AnyObject = msg_send![
        message_string,
        initWithBytes: message.as_ptr() as *const std::ffi::c_void
        length: message.len()
        encoding: 4_usize  // NSUTF8StringEncoding
    ];

    let _: () = msg_send![alert, setMessageText: message_string];

    // Add OK button
    let ok_string: *mut AnyObject = msg_send![ns_string_class, alloc];
    let ok_string: *mut AnyObject = msg_send![
        ok_string,
        initWithBytes: "OK".as_ptr() as *const std::ffi::c_void
        length: 2_usize
        encoding: 4_usize
    ];
    let _: () = msg_send![alert, addButtonWithTitle: ok_string];

    // Set alert style to informational
    let _: () = msg_send![alert, setAlertStyle: 1_isize]; // NSAlertStyleInformational = 1

    // Show the alert
    let _: () = msg_send![alert, runModal];
}
