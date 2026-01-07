//! macOS system actions (namespace: "os")

use crate::actions_sync;

// Define all macOS actions using the sync macro
// Actions are automatically registered with the "os" namespace
actions_sync!("os", {
    test_notification,
    test_swift_menus,
    execute_menu_item,
    test_app_info,
    reload_config,
    dump_app_menus,
    test_menu_click,
    list_running_apps,
    focus_protools,
    launch_application,
    list_window_buttons,
    click_window_button,
    display_window_text,
    test_input_dialog,
    rapid_pw,
    test_pw,
    list_window_titles,
    test_keystroke,
    shell_script,
    test_window,
    test_modal_window,
    test_text_window,
});
