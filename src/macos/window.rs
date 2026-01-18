//! Native macOS window for displaying text
//!
//! This module provides a simple text display window using native Objective-C
//! via objc2. It creates an NSWindow with NSTextView to show text content.
//!
//! STATUS: EXPERIMENTAL - Learning example for Objective-C patterns

use anyhow::Result as R;
use objc2::msg_send;
use objc2::rc::autoreleasepool;
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
    pub unsafe fn create_window(&self, frame: NSRect, title: &str) -> R<*mut AnyObject> {
        unsafe {
            let window_class = self.get_class("NSWindow")?;
            let window: *mut AnyObject = msg_send![window_class, alloc];

            // initWithContentRect:styleMask:backing:defer:
            //   - styleMask: 15 = Titled + Closable + Miniaturizable + Resizable
            //   - backing: 2 = NSBackingStoreBuffered
            let window: *mut AnyObject = msg_send![
                window,
                initWithContentRect: frame
                styleMask: 15_usize
                backing: 2_usize
                defer: false
            ];

            if window.is_null() {
                anyhow::bail!("Failed to create NSWindow");
            }

            // Set window title
            let title_str = self.create_nsstring(title)?;
            let _: () = msg_send![window, setTitle: title_str];
            // Auto-release the window when closed to prevent memory leaks and crashes
            let _: () = msg_send![window, setReleasedWhenClosed: true];

            Ok(window)
        }
    }

    /// Create an NSScrollView with frame
    ///
    /// # Example
    /// ```ignore
    /// let scroll_view = os.create_scroll_view(frame)?;
    /// ```
    pub unsafe fn create_scroll_view(&self, frame: NSRect) -> R<*mut AnyObject> {
        unsafe {
            let scroll_view_class = self.get_class("NSScrollView")?;
            let scroll_view: *mut AnyObject = msg_send![scroll_view_class, alloc];
            let scroll_view: *mut AnyObject = msg_send![scroll_view, initWithFrame: frame];

            if scroll_view.is_null() {
                anyhow::bail!("Failed to create NSScrollView");
            }

            // Configure scroll view
            let _: () = msg_send![scroll_view, setHasVerticalScroller: true];
            let _: () = msg_send![scroll_view, setHasHorizontalScroller: true];
            let _: () = msg_send![scroll_view, setAutohidesScrollers: true];
            let _: () = msg_send![scroll_view, setBorderType: 0_usize]; // NSNoBorder

            Ok(scroll_view)
        }
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
    ) -> R<*mut AnyObject> {
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
    pub unsafe fn show_text_window(&self, text: &str) -> R<()> {
        // Wrap in autorelease pool for proper memory management
        unsafe {
            autoreleasepool(|_pool| {
                // Create window
                let frame = MacOSSession::rect(100.0, 100.0, 800.0, 600.0);
                let window = self.create_window(frame, "Output")?;

                // Create scroll view to hold the text
                let scroll_frame = MacOSSession::rect(0.0, 0.0, 800.0, 600.0);
                let scroll_view = self.create_scroll_view(scroll_frame)?;

                // Get actual content size (accounts for scrollers)
                let content_size: NSSize = msg_send![scroll_view, contentSize];
                let text_frame = NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: content_size,
                };

                // Create text view with the content
                let text_view = self.create_text_view(text_frame, text, false)?;

                // Assemble the view hierarchy
                let _: () = msg_send![scroll_view, setDocumentView: text_view];
                let content_view: *mut AnyObject = msg_send![window, contentView];
                let _: () = msg_send![content_view, addSubview: scroll_view];

                // Show the window
                self.show_window(window);

                log::info!("Text window displayed successfully");
                Ok(())
            })
        }
    }

    /// Show a simple message dialog
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// os.show_message_dialog("Operation completed successfully")?;
    pub unsafe fn show_message_dialog(&self, message: &str) -> R<()> {
        // Wrap in autorelease pool for proper memory management
        unsafe {
            autoreleasepool(|_pool| {
                self.show_alert("Message", message, &["OK"])?;
                Ok(())
            })
        }
    }
}

use std::process::Command;

/// Show a modal input dialog and return the user's text input
///
/// This function displays a native macOS input dialog using osascript.
/// The dialog is modal and blocks until the user responds.
///
/// # Parameters
/// * `title` - The dialog title
/// * `prompt` - Optional prompt text (if None, uses title)
/// * `default_value` - Optional pre-filled text in the input field
///
/// # Returns
/// * `Ok(Some(String))` - User clicked OK with text (even if empty)
/// * `Ok(None)` - User clicked Cancel or closed dialog
/// * `Err(_)` - System error (osascript failure, etc.)
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
) -> R<Option<String>> {
    log::info!("Showing input dialog: '{}'", title);

    // Escape strings for AppleScript
    let escaped_title = title.replace("\"", "\\\"");
    let escaped_prompt = prompt.unwrap_or(title).replace("\"", "\\\"");
    let escaped_default = default_value.unwrap_or("").replace("\"", "\\\"");

    // Build AppleScript command
    let script = format!(
        "display dialog \"{}\" default answer \"{}\" with title \"{}\" buttons {{\"Cancel\", \"OK\"}} default button \"OK\"",
        escaped_prompt, escaped_default, escaped_title
    );

    // Execute osascript
    let output = Command::new("osascript").arg("-e").arg(&script).output()?;

    // Check if user cancelled (exit code 1 with "User canceled" in stderr)
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") {
            log::info!("User cancelled dialog");
            return Ok(None);
        }
        anyhow::bail!("osascript failed: {}", stderr);
    }

    // Parse output: "button returned:OK, text returned:user input"
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract text between "text returned:" and end of line/string
    if let Some(text_start) = stdout.find("text returned:") {
        let text = stdout[text_start + 14..].trim().to_string();
        log::info!("User entered: '{}'", text);
        Ok(Some(text))
    } else {
        log::info!("User clicked OK with empty input");
        Ok(Some(String::new()))
    }
}
