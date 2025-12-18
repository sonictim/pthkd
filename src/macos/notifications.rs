use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};
use std::process::Command;
use std::ptr;

/// Show a macOS notification
///
/// Tries native NSUserNotificationCenter first (uses app icon),
/// falls back to osascript if native fails (more reliable)
pub fn show_notification(message: &str) {
    let message = message.to_string();
    std::thread::spawn(move || {
        // Try native notification first (uses app icon)
        if let Err(e) = show_notification_native(&message) {
            log::debug!("Native notification failed ({}), falling back to osascript", e);

            // Fall back to osascript (more reliable for menu bar apps)
            if let Err(e) = show_notification_osascript(&message) {
                log::error!("Failed to show notification: {}", e);
            }
        }
    });
}

/// Show notification using NSUserNotificationCenter (native API)
///
/// This will use the app's icon from the bundle instead of the AppleScript icon
fn show_notification_native(message: &str) -> Result<(), String> {
    unsafe {
        // Get NSUserNotificationCenter class
        let notification_center_class = AnyClass::get("NSUserNotificationCenter")
            .ok_or("Failed to get NSUserNotificationCenter class")?;

        // Get the default notification center
        let center: *mut AnyObject = msg_send![notification_center_class, defaultUserNotificationCenter];
        if center.is_null() {
            return Err("Failed to get default notification center".to_string());
        }

        // Create a new notification
        let notification_class = AnyClass::get("NSUserNotification")
            .ok_or("Failed to get NSUserNotification class")?;
        let notification: *mut AnyObject = msg_send![notification_class, alloc];
        let notification: *mut AnyObject = msg_send![notification, init];

        if notification.is_null() {
            return Err("Failed to create notification".to_string());
        }

        // Create NSString for title
        let title = create_nsstring("ProTools Hotkey Daemon");
        if title.is_null() {
            return Err("Failed to create title string".to_string());
        }

        // Create NSString for message
        let message_str = create_nsstring(message);
        if message_str.is_null() {
            return Err("Failed to create message string".to_string());
        }

        // Set notification properties
        let _: () = msg_send![notification, setTitle: title];
        let _: () = msg_send![notification, setInformativeText: message_str];

        // Deliver the notification
        let _: () = msg_send![center, deliverNotification: notification];

        log::debug!("Native notification delivered: {}", message);
        Ok(())
    }
}

/// Show notification using osascript AppleScript (fallback)
///
/// More reliable for menu bar apps, but uses AppleScript icon
fn show_notification_osascript(message: &str) -> Result<(), String> {
    // Escape single quotes in message for AppleScript
    let escaped_message = message.replace("'", "\\'");

    let script = format!(
        "display notification \"{}\" with title \"ProTools Hotkey Daemon\"",
        escaped_message
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed: {}", stderr));
    }

    log::debug!("osascript notification delivered: {}", message);
    Ok(())
}

/// Helper to create an NSString from a Rust &str
unsafe fn create_nsstring(s: &str) -> *mut AnyObject {
    let ns_string_class = match AnyClass::get("NSString") {
        Some(class) => class,
        None => return ptr::null_mut(),
    };

    let ns_string: *mut AnyObject = msg_send![ns_string_class, alloc];
    let ns_string: *mut AnyObject = msg_send![
        ns_string,
        initWithBytes: s.as_ptr() as *const std::ffi::c_void
        length: s.len()
        encoding: 4_usize  // NSUTF8StringEncoding = 4
    ];

    ns_string
}
