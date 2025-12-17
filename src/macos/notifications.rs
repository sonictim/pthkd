use std::process::Command;

/// Show a macOS notification using osascript (AppleScript)
///
/// This uses the native macOS notification system via osascript,
/// which is simple, reliable, and works on all macOS versions.
pub fn show_notification(message: &str) {
    let message = message.to_string();
    std::thread::spawn(move || {
        if let Err(e) = show_notification_osascript(&message) {
            log::error!("Failed to show notification: {}", e);
        }
    });
}

/// Show notification using osascript AppleScript
fn show_notification_osascript(message: &str) -> Result<(), String> {
    // Escape single quotes in message for AppleScript
    let escaped_message = message.replace("'", "\\'");

    let script = format!(
        "display notification \"{}\" with title \"Hotkey Daemon\"",
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

    log::debug!("Notification delivered: {}", message);
    Ok(())
}
