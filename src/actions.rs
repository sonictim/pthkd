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
    use std::time::Instant;

    log::info!("=== App Focus Information ===");
    println!("=== App Focus Information ===");

    // Benchmark get_current_app()
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = app_info::get_current_app();
    }
    let elapsed = start.elapsed();
    let msg = format!(
        "⏱️  get_current_app() benchmark: {:?} per call (1000 calls in {:?})",
        elapsed / 1000,
        elapsed
    );
    log::info!("{}", msg);
    println!("{}", msg);

    // Get current app (no permissions needed)
    match app_info::get_current_app() {
        Ok(app_name) => {
            log::info!("Current App: {}", app_name);
            println!("Current App: {}", app_name);
        }
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

pub fn reload_config() {
    use crate::config::{config_to_hotkeys, load_config};
    use crate::hotkey::HOTKEYS;

    log::info!("Reloading config from config.toml...");

    // Load and parse config
    let config = match load_config("config.toml") {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to reload config: {:#}", e);
            show_notification("❌ Failed to reload config");
            return;
        }
    };

    // Convert to hotkeys
    let hotkeys = match config_to_hotkeys(config) {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to parse config: {:#}", e);
            show_notification("❌ Failed to parse config");
            return;
        }
    };

    // Log registered hotkeys
    log::info!("Reloaded {} hotkeys:", hotkeys.len());
    for hotkey in &hotkeys {
        log::info!("  - {} => {}", hotkey.chord.describe(), hotkey.action_name);
    }

    // Update the global hotkey registry
    if let Some(hotkeys_mutex) = HOTKEYS.get() {
        *hotkeys_mutex.lock().unwrap() = hotkeys;
        log::info!("✅ Config reloaded successfully!");
        show_notification("✅ Config reloaded!");
    } else {
        log::error!("HOTKEYS not initialized - cannot reload");
        show_notification("❌ HOTKEYS not initialized");
    }
}

pub fn dump_app_menus() {
    use crate::macos::menu::get_app_menus;

    // Get menu structure for Pro Tools
    let app_name = "Pro Tools";
    log::info!("Getting menu structure for {}...", app_name);

    match get_app_menus(app_name) {
        Ok(menu_bar) => {
            let json = serde_json::to_string_pretty(&menu_bar).unwrap();
            log::info!("Menu structure for {}:\n{}", app_name, json);
            println!("Menu structure for {}:\n{}", app_name, json);
            show_notification(&format!("✅ {} menus logged!", app_name));
        }
        Err(e) => {
            log::error!("Failed to get menus for {}: {:#}", app_name, e);
            show_notification(&format!("❌ Failed: {}", e));
        }
    }
}

pub fn test_menu_click() {
    use crate::macos::menu::run_menu_item;

    log::info!("Testing menu click...");

    // Test with a simple menu item - adjust this to whatever you want to test
    match run_menu_item("Soundminer_Intel", &["DAW", "Pro Tools"]) {
        Ok(_) => {
            log::info!("✅ Menu click succeeded!");
            show_notification("✅ Menu clicked!");
        }
        Err(e) => {
            log::error!("Failed to click menu: {:#}", e);
            show_notification(&format!("❌ Failed: {}", e));
        }
    }
    match run_menu_item("Soundminer_Intel", &["Transfer", "Pro Tools"]) {
        Ok(_) => {
            log::info!("✅ Menu click succeeded!");
            show_notification("✅ Menu clicked!");
        }
        Err(e) => {
            log::error!("Failed to click menu: {:#}", e);
            show_notification(&format!("❌ Failed: {}", e));
        }
    }
}

pub fn list_running_apps() {
    use crate::macos::app_info::get_all_running_applications;

    log::info!("Getting list of running applications...");

    match get_all_running_applications() {
        Ok(apps) => {
            log::info!("Running applications ({}):", apps.len());
            println!("\n=== Running Applications ({}) ===", apps.len());
            for app in &apps {
                log::info!("  - {}", app);
                println!("  - {}", app);
            }
            show_notification(&format!("✅ Found {} running apps", apps.len()));
        }
        Err(e) => {
            log::error!("Failed to get running applications: {:#}", e);
            show_notification(&format!("❌ Failed: {}", e));
        }
    }
}

pub fn focus_protools() {
    use crate::macos::app_info::focus_application;

    log::info!("Focusing Pro Tools...");

    match focus_application("Pro Tools") {
        Ok(_) => {
            log::info!("✅ Pro Tools focused successfully!");
            show_notification("✅ Pro Tools focused!");
        }
        Err(e) => {
            log::error!("Failed to focus Pro Tools: {:#}", e);
            show_notification(&format!("❌ Failed: {}", e));
        }
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
    registry.insert("reload_config", reload_config as fn());
    registry.insert("dump_app_menus", dump_app_menus as fn());
    registry.insert("test_menu_click", test_menu_click as fn());
    registry.insert("list_running_apps", list_running_apps as fn());
    registry.insert("focus_protools", focus_protools as fn());
    registry.insert(
        "spot_to_protools",
        crate::soundminer::spot_to_protools as fn(),
    );

    // Add more actions as needed:
    // registry.insert("my_custom", my_custom as fn());

    registry
}
