use crate::macos::show_notification;
use std::collections::HashMap;

// ====== Action Functions ======
// Define your custom action functions here

pub fn test_notification() {
    show_notification("CMD+Shift+K pressed!");
}

pub fn example_two() {
    show_notification("CMD+Option+R pressed!");
}

pub fn test_keystroke() {
    log::info!("Testing global keystroke - sending CMD+F1");
    match crate::macos::keystroke::send_keystroke(&["cmd", "f1"]) {
        Ok(_) => log::info!("Keystroke sent successfully"),
        Err(e) => log::error!("Failed to send keystroke: {}", e),
    }
}

pub fn test_app_info() {
    use crate::macos::app_info;

    log::info!("=== App Focus Information ===");

    // Get current app (no permissions needed)
    match app_info::get_current_app() {
        Ok(app_name) => println!("Current App: {}", app_name),
        Err(e) => log::error!("Failed to get app: {}", e),
    }

    // Check if we have accessibility permissions
    if !app_info::has_accessibility_permission() {
        log::warn!(
            "⚠️  Accessibility permissions not granted! \
            Enable in System Preferences > Security & Privacy > Accessibility"
        );
        log::info!("(Window title and text field detection require accessibility permissions)");
        return;
    }

    // Get window title (requires permissions)
    match app_info::get_app_window() {
        Ok(title) => println!("Window Title: {}", title),
        Err(e) => log::error!("Failed to get window: {}", e),
    }

    // Check if in text field (requires permissions)
    match app_info::is_in_text_field() {
        Ok(is_text) => {
            if is_text {
                println!("Text Field: ✅ Yes (cursor is in a text entry field)");
            } else {
                println!("Text Field: ❌ No (not in a text field)");
            }
        }
        Err(e) => log::error!("Failed to check text field: {}", e),
    }
}

// Add more actions as needed
// pub fn my_custom() {
//     println!("Custom action!");
// }

// ====== Action Registry ======

/// Returns a registry mapping action names to their function pointers
pub fn get_action_registry() -> HashMap<&'static str, fn()> {
    let mut registry = HashMap::new();

    // Register all actions here
    registry.insert("test_notification", test_notification as fn());
    registry.insert("example_two", example_two as fn());
    registry.insert("test_keystroke", test_keystroke as fn());
    registry.insert("test_app_info", test_app_info as fn());

    // Add more actions as needed:
    // registry.insert("my_custom", my_custom as fn());

    registry
}
