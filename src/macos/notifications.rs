/// Show a macOS notification in a separate thread to avoid blocking
///
/// This spawns a new thread to display the notification, ensuring the
/// event tap callback is not blocked while the notification is shown.
pub fn show_notification(message: &str) {
    let message = message.to_string();
    std::thread::spawn(move || {
        mac_notification_sys::send_notification(
            "Hotkey Daemon",
            None, // No subtitle
            &message,
            None  // No sound (use Some("Basso") for sound)
        ).ok();
    });
}
