/// Show a macOS notification using osascript
///
/// After the async refactor, mac_notification_sys hangs (likely needs main thread/NSRunLoop).
/// osascript is more reliable and doesn't have threading requirements.
pub fn show_notification(message: &str) {
    let message = message.to_string();
    std::thread::spawn(move || {
        let escaped = message.replace("\"", "\\\"").replace("\\", "\\\\");
        let script = format!(r#"display notification "{}" with title "Hotkey Daemon""#, escaped);

        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output();
    });
}
