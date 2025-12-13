//! macOS system actions (namespace: "os")

use crate::actions_sync;

// Define all macOS actions using the sync macro
// Actions are automatically registered with the "os" namespace
actions_sync!("os", {
    test_notification,
    test_keystroke,
    test_app_info,
    reload_config,
    dump_app_menus,
    test_menu_click,
    list_running_apps,
    focus_protools,
    list_window_buttons,
    click_window_button,
    display_window_text,
    test_input_dialog,
    fast_pw,
});
