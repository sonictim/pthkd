use super::session::MacOSSession;
use objc2::msg_send;
use objc2::runtime::AnyObject;
use std::process::Command;

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
        let os = MacOSSession::global();

        // Get NSUserNotificationCenter class using session
        let notification_center_class = os
            .get_class("NSUserNotificationCenter")
            .map_err(|e| format!("Failed to get NSUserNotificationCenter class: {}", e))?;

        // Get the default notification center
        let center: *mut AnyObject = msg_send![notification_center_class, defaultUserNotificationCenter];
        if center.is_null() {
            return Err("Failed to get default notification center".to_string());
        }

        // Create a new notification using session
        let notification_class = os
            .get_class("NSUserNotification")
            .map_err(|e| format!("Failed to get NSUserNotification class: {}", e))?;
        let notification = os
            .alloc_init(notification_class)
            .map_err(|e| format!("Failed to create notification: {}", e))?;

        // Create NSStrings using session methods
        let title = os
            .create_nsstring("ProTools Hotkey Daemon")
            .map_err(|e| format!("Failed to create title string: {}", e))?;

        let message_str = os
            .create_nsstring(message)
            .map_err(|e| format!("Failed to create message string: {}", e))?;

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
