//! macOS system command implementations

use crate::params::Params;
use super::{show_notification, app_info, menu, keystroke};
use anyhow::Result;

// ============================================================================
// Command Implementations
// ============================================================================

pub fn test_notification(_params: &Params) -> Result<()> {
    show_notification("CMD+Shift+K pressed!");
    Ok(())
}

pub fn test_keystroke(_params: &Params) -> Result<()> {
    log::info!("Testing global keystroke - sending CMD+F1");
    keystroke::send_keystroke(&["cmd", "f1"])?;
    log::info!("Keystroke sent successfully");
    Ok(())
}

pub fn test_app_info(_params: &Params) -> Result<()> {
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
        return Ok(());
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

    Ok(())
}

pub fn reload_config(_params: &Params) -> Result<()> {
    use crate::config::{config_to_hotkeys, load_config};
    use crate::hotkey::HOTKEYS;
    use anyhow::{Context, bail};

    log::info!("Reloading config from config.toml...");

    // Load and parse config
    let config = load_config("config.toml")
        .context("Failed to load config.toml")?;

    // Convert to hotkeys
    let hotkeys = config_to_hotkeys(config)
        .context("Failed to parse config")?;

    // Log registered hotkeys
    log::info!("Reloaded {} hotkeys:", hotkeys.len());
    for hotkey in &hotkeys {
        log::info!("  - {} => {}", hotkey.chord.describe(), hotkey.action_name);
    }

    // Update the global hotkey registry
    if let Some(hotkeys_mutex) = HOTKEYS.get() {
        *hotkeys_mutex.lock().unwrap() = hotkeys;
        log::info!("✅ Config reloaded successfully!");
        Ok(())
    } else {
        log::error!("HOTKEYS not initialized - cannot reload");
        bail!("HOTKEYS not initialized")
    }
}

pub fn dump_app_menus(_params: &Params) -> Result<()> {
    use anyhow::Context;

    // Get menu structure for Pro Tools
    let app_name = "Pro Tools";
    log::info!("Getting menu structure for {}...", app_name);

    let menu_bar = menu::get_app_menus(app_name)
        .context(format!("Failed to get menus for {}", app_name))?;

    let json = serde_json::to_string_pretty(&menu_bar)?;
    log::info!("Menu structure for {}:\n{}", app_name, json);
    println!("Menu structure for {}:\n{}", app_name, json);

    Ok(())
}

pub fn test_menu_click(_params: &Params) -> Result<()> {
    log::info!("Testing menu click...");

    // Test with a simple menu item - adjust this to whatever you want to test
    menu::run_menu_item("Soundminer_Intel", &["DAW", "Pro Tools"])?;
    log::info!("✅ Menu click 1 succeeded!");

    menu::run_menu_item("Soundminer_Intel", &["Transfer", "Pro Tools"])?;
    log::info!("✅ Menu click 2 succeeded!");

    Ok(())
}

pub fn list_running_apps(_params: &Params) -> Result<()> {
    use anyhow::Context;

    log::info!("Getting list of running applications...");

    let apps = app_info::get_all_running_applications()
        .context("Failed to get running applications")?;

    log::info!("Running applications ({}):", apps.len());
    println!("\n=== Running Applications ({}) ===", apps.len());
    for app in &apps {
        log::info!("  - {}", app);
        println!("  - {}", app);
    }

    Ok(())
}

pub fn focus_protools(_params: &Params) -> Result<()> {
    use anyhow::Context;

    log::info!("Focusing Pro Tools...");
    app_info::focus_application("Pro Tools")
        .context("Failed to focus Pro Tools")?;
    log::info!("✅ Pro Tools focused successfully!");

    Ok(())
}

pub fn list_window_buttons(params: &Params) -> Result<()> {
    use anyhow::Context;

    let app_name = params.get_string("app", "Pro Tools");
    let window_name = params.get_string("window", "");

    log::info!("Listing buttons in window '{}' of app '{}'...",
               if window_name.is_empty() { "<focused>" } else { &window_name },
               app_name);

    let buttons = super::ui_elements::get_window_buttons(&app_name, &window_name)
        .context("Failed to get window buttons")?;

    log::info!("Found {} buttons:", buttons.len());
    println!("\n=== Buttons in window ===");
    for (i, button) in buttons.iter().enumerate() {
        log::info!("  {}. {}", i + 1, button);
        println!("  {}. {}", i + 1, button);
    }

    Ok(())
}

pub fn click_window_button(params: &Params) -> Result<()> {
    use anyhow::Context;

    let app_name = params.get_string("app", "Pro Tools");
    let window_name = params.get_string("window", "");
    let button_name = params.get_string("button", "");

    if button_name.is_empty() {
        anyhow::bail!("button parameter is required");
    }

    log::info!("Clicking button '{}' in window '{}' of app '{}'...",
               button_name,
               if window_name.is_empty() { "<focused>" } else { &window_name },
               app_name);

    super::ui_elements::click_button(&app_name, &window_name, &button_name)
        .context("Failed to click button")?;

    Ok(())
}
