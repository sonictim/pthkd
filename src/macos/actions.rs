//! macOS system actions (namespace: "os")

use crate::actions;

// Define all macOS actions using the unified async macro
// Actions are automatically registered with the "os" namespace
actions!("os", {
    test_notification,
    test_keystroke,
    test_app_info,
    reload_config,
    list_running_apps,
    focus_protools,
    list_window_buttons,
    click_window_button,
    display_window_text,
    test_input_dialog,
});
