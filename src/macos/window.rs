//! Native macOS window for displaying text
//!
//! This module provides a simple text display window using native Objective-C
//! via objc2. It creates an NSWindow with NSTextView to show text content.
//!
//! STATUS: EXPERIMENTAL - Learning example for Objective-C patterns

use anyhow::Result;
use objc2::msg_send;
use objc2::runtime::AnyObject;
use std::ptr;

// Import session types
use super::session::{MacOSSession, NSPoint, NSRect, NSSize};

// ============================================================================
// Window Extensions for MacOSSession
// ============================================================================

impl MacOSSession {
    /// Create an NSWindow with frame and title
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// let frame = MacOSSession::rect(100.0, 100.0, 800.0, 600.0);
    /// let window = os.create_window(frame, "My Window")?;
    /// ```
    pub unsafe fn create_window(&self, frame: NSRect, title: &str) -> Result<*mut AnyObject> {
        let window_class = unsafe { self.get_class("NSWindow")? };
        let window: *mut AnyObject = unsafe { msg_send![window_class, alloc] };

        // initWithContentRect:styleMask:backing:defer:
        //   - styleMask: 15 = Titled + Closable + Miniaturizable + Resizable
        //   - backing: 2 = NSBackingStoreBuffered
        let window: *mut AnyObject = unsafe {
            msg_send![
                window,
                initWithContentRect: frame
                styleMask: 15_usize
                backing: 2_usize
                defer: false
            ]
        };

        if window.is_null() {
            anyhow::bail!("Failed to create NSWindow");
        }

        // Set window title
        let title_str = unsafe { self.create_nsstring(title)? };
        unsafe {
            let _: () = msg_send![window, setTitle: title_str];
            let _: () = msg_send![window, setReleasedWhenClosed: false];
        }

        Ok(window)
    }

    /// Create an NSScrollView with frame
    ///
    /// # Example
    /// ```ignore
    /// let scroll_view = os.create_scroll_view(frame)?;
    /// ```
    pub unsafe fn create_scroll_view(&self, frame: NSRect) -> Result<*mut AnyObject> {
        let scroll_view_class = unsafe { self.get_class("NSScrollView")? };
        let scroll_view: *mut AnyObject = unsafe { msg_send![scroll_view_class, alloc] };
        let scroll_view: *mut AnyObject = unsafe { msg_send![scroll_view, initWithFrame: frame] };

        if scroll_view.is_null() {
            anyhow::bail!("Failed to create NSScrollView");
        }

        // Configure scroll view
        unsafe {
            let _: () = msg_send![scroll_view, setHasVerticalScroller: true];
            let _: () = msg_send![scroll_view, setHasHorizontalScroller: true];
            let _: () = msg_send![scroll_view, setAutohidesScrollers: true];
            let _: () = msg_send![scroll_view, setBorderType: 0_usize]; // NSNoBorder
        }

        Ok(scroll_view)
    }

    /// Create an NSTextView with frame and content
    ///
    /// # Example
    /// ```ignore
    /// let text_view = os.create_text_view(frame, "Hello World", false)?;
    /// ```
    pub unsafe fn create_text_view(
        &self,
        frame: NSRect,
        text: &str,
        editable: bool,
    ) -> Result<*mut AnyObject> {
        let text_view_class = self.get_class("NSTextView")?;
        let text_view: *mut AnyObject = unsafe { msg_send![text_view_class, alloc] };
        let text_view: *mut AnyObject = unsafe { msg_send![text_view, initWithFrame: frame] };

        if text_view.is_null() {
            anyhow::bail!("Failed to create NSTextView");
        }

        // Configure text view
        unsafe {
            let _: () = msg_send![text_view, setEditable: editable];
            let _: () = msg_send![text_view, setSelectable: true];
            let _: () = msg_send![text_view, setHorizontallyResizable: false];
            let _: () = msg_send![text_view, setVerticallyResizable: true];

            // Configure text container for wrapping
            let text_container: *mut AnyObject = msg_send![text_view, textContainer];
            let _: () = msg_send![text_container, setContainerSize: NSSize {
                width: frame.size.width,
                height: 10000000.0
            }];
            let _: () = msg_send![text_container, setWidthTracksTextView: true];

            // Set text content
            let content = self.create_nsstring(text)?;
            let _: () = msg_send![text_view, setString: content];
        }

        Ok(text_view)
    }

    /// Show and activate a window
    ///
    /// # Example
    /// ```ignore
    /// os.show_window(window);
    /// ```
    pub unsafe fn show_window(&self, window: *mut AnyObject) {
        unsafe {
            let _: () = msg_send![window, makeKeyAndOrderFront: ptr::null::<AnyObject>()];
        }
    }

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
        // Create window
        let frame = MacOSSession::rect(100.0, 100.0, 800.0, 600.0);
        let window = unsafe { self.create_window(frame, "Output")? };

        // Create scroll view to hold the text
        let scroll_frame = MacOSSession::rect(0.0, 0.0, 800.0, 600.0);
        let scroll_view = unsafe { self.create_scroll_view(scroll_frame)? };

        // Get actual content size (accounts for scrollers)
        let content_size: NSSize = unsafe { msg_send![scroll_view, contentSize] };
        let text_frame = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: content_size,
        };

        // Create text view with the content
        let text_view = unsafe { self.create_text_view(text_frame, text, false)? };

        // Assemble the view hierarchy
        unsafe {
            let _: () = msg_send![scroll_view, setDocumentView: text_view];
            let content_view: *mut AnyObject = msg_send![window, contentView];
            let _: () = msg_send![content_view, addSubview: scroll_view];
        }

        // Show the window
        unsafe { self.show_window(window) };

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
        unsafe { self.show_alert("Message", message, &["OK"])? };
        Ok(())
    }
}
