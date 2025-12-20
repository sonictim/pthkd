//! Native macOS window for displaying text
//!
//! This module provides a simple text display window using native Objective-C
//! via objc2. It creates an NSWindow with NSTextView to show text content.
//!
//! STATUS: EXPERIMENTAL - Learning example for Objective-C patterns

use anyhow::Result;
use objc2::runtime::AnyObject;
use objc2::msg_send;
use std::ptr;

// Import session types
use super::session::{MacOSSession, NSRect, NSPoint, NSSize};

// ============================================================================
// Window Extensions for MacOSSession
// ============================================================================

impl MacOSSession {
    /// Shows a simple window displaying the provided text
    ///
    /// Creates a native macOS window (NSWindow) with an NSTextView containing
    /// the provided text. The window is non-editable but text is selectable.
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// os.show_text_window("Hello World!")?;
    /// ```
    pub unsafe fn show_text_window(&self, text: &str) -> Result<()> {
        // 1. Get NSWindow class and create window
        let window_class = self.get_class("NSWindow")?;
        let window: *mut AnyObject = msg_send![window_class, alloc];

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
        let title = self.create_nsstring("Output")?;
        let _: () = msg_send![window, setTitle: title];

        // Keep window alive (don't release when closed)
        let _: () = msg_send![window, setReleasedWhenClosed: false];

        // 3. Create NSScrollView to hold the text view
        let scroll_view_class = self.get_class("NSScrollView")?;
        let scroll_view: *mut AnyObject = msg_send![scroll_view_class, alloc];
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
        let text_view_class = self.get_class("NSTextView")?;
        let text_view: *mut AnyObject = msg_send![text_view_class, alloc];

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
        let content = self.create_nsstring(text)?;
        let _: () = msg_send![text_view, setString: content];

        // 7. Add text view as document view of scroll view
        let _: () = msg_send![scroll_view, setDocumentView: text_view];

        // 8. Add scroll view to window's content view
        let content_view: *mut AnyObject = msg_send![window, contentView];
        let _: () = msg_send![content_view, addSubview: scroll_view];

        // 9. Show window and make it active
        let _: () = msg_send![window, makeKeyAndOrderFront: ptr::null::<AnyObject>()];

        log::info!("Text window displayed successfully");

        Ok(())
    }

    /// Show a simple message dialog
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// os.show_message_dialog("Operation completed successfully")?;
    /// ```
    pub unsafe fn show_message_dialog(&self, message: &str) -> Result<()> {
        self.show_alert("Message", message, &["OK"])?;
        Ok(())
    }
}
