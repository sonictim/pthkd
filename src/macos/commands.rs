//! macOS system command implementations
//!
//! Phase 1: Commands are now async and take &mut MacOSSession parameter

use crate::params::Params;
use super::{show_notification, MacOSSession};
use anyhow::Result;

// ============================================================================
// Command Implementations
// ============================================================================

pub async fn test_notification(_macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    log::info!("test_notification: Showing notification");
    show_notification("CMD+Shift+K pressed!");
    log::info!("test_notification: Notification shown");

    // Give the notification thread a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    Ok(())
}

pub async fn test_keystroke(macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    log::info!("Testing global keystroke - sending CMD+F1");
    macos.send_keystroke(0x7A, &["cmd"]).await?; // F1 key with Cmd
    log::info!("Keystroke sent successfully");
    Ok(())
}

pub async fn test_app_info(macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    use std::time::Instant;

    log::info!("=== App Focus Information ===");
    println!("=== App Focus Information ===");

    // Benchmark get_focused_app()
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = macos.get_focused_app().await;
    }
    let elapsed = start.elapsed();
    let msg = format!(
        "⏱️  get_focused_app() benchmark: {:?} per call (1000 calls in {:?})",
        elapsed / 1000,
        elapsed
    );
    log::info!("{}", msg);
    println!("{}", msg);

    // Get current app
    match macos.get_focused_app().await {
        Ok(app_name) => {
            log::info!("Current App: {}", app_name);
            println!("Current App: {}", app_name);
        }
        Err(e) => log::error!("Failed to get app: {}", e),
    }

    // Check if we have accessibility permissions
    if !macos.has_accessibility_permission() {
        log::warn!(
            "⚠️  Accessibility permissions not granted! \
            Enable in System Preferences > Security & Privacy > Accessibility"
        );
        log::info!("(Window title and text field detection require accessibility permissions)");
        return Ok(());
    }

    // Get window title (requires permissions)
    match macos.get_focused_window().await {
        Ok(title) => println!("Window Title: {}", title),
        Err(e) => log::error!("Failed to get window: {}", e),
    }

    // Check if in text field (requires permissions)
    match macos.is_in_text_field().await {
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

pub async fn reload_config(_macos: &mut MacOSSession, _params: &Params) -> Result<()> {
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

pub async fn list_running_apps(macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    use anyhow::Context;

    log::info!("Getting list of running applications...");

    let apps = macos.get_running_apps().await
        .context("Failed to get running applications")?;

    log::info!("Running applications ({}):", apps.len());
    println!("\n=== Running Applications ({}) ===", apps.len());
    for app in &apps {
        log::info!("  - {}", app);
        println!("  - {}", app);
    }

    Ok(())
}

pub async fn focus_protools(macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    use anyhow::Context;

    log::info!("Focusing Pro Tools...");
    macos.focus_app("Pro Tools").await
        .context("Failed to focus Pro Tools")?;
    log::info!("✅ Pro Tools focused successfully!");

    Ok(())
}

pub async fn list_window_buttons(macos: &mut MacOSSession, params: &Params) -> Result<()> {
    use anyhow::Context;

    let app_name = params.get_str("app", "Pro Tools");
    let window_name = params.get_str("window", "");

    log::info!("Listing buttons in window '{}' of app '{}'...",
               if window_name.is_empty() { "<focused>" } else { window_name },
               app_name);

    let buttons = macos.get_window_buttons(app_name, window_name).await
        .context("Failed to get window buttons")?;

    log::info!("Found {} buttons:", buttons.len());
    println!("\n=== Buttons in window ===");
    for (i, button) in buttons.iter().enumerate() {
        log::info!("  {}. {}", i + 1, button);
        println!("  {}. {}", i + 1, button);
    }

    Ok(())
}

pub async fn click_window_button(macos: &mut MacOSSession, params: &Params) -> Result<()> {
    use anyhow::Context;

    let app_name = params.get_str("app", "Pro Tools");
    let window_name = params.get_str("window", "");
    let button_name = params.get_str("button", "");

    if button_name.is_empty() {
        anyhow::bail!("button parameter is required");
    }

    log::info!("Clicking button '{}' in window '{}' of app '{}'...",
               button_name,
               if window_name.is_empty() { "<focused>" } else { window_name },
               app_name);

    macos.click_button(app_name, window_name, button_name).await
        .context("Failed to click button")?;

    Ok(())
}

pub async fn display_window_text(macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    log::info!("Getting text from focused window...");

    // Get current app
    let app_name = macos.get_focused_app().await?;
    log::info!("Current app: {}", app_name);
    println!("Current app: {}", app_name);

    // Get text from focused window (empty string = focused window)
    match macos.get_window_text(&app_name, "").await {
        Ok(text_elements) => {
            log::info!("Found {} text elements", text_elements.len());
            println!("\n=== Window Text ({} elements) ===", text_elements.len());
            for (i, text) in text_elements.iter().enumerate() {
                log::info!("  {}. {}", i + 1, text);
                println!("  {}. {}", i + 1, text);
            }
        }
        Err(e) => {
            log::error!("Failed to get window text: {}", e);
            println!("❌ Failed to get window text: {}", e);
        }
    }

    Ok(())
}

pub async fn test_input_dialog(macos: &mut MacOSSession, _params: &Params) -> Result<()> {
    log::info!("=== test_input_dialog: START ===");

    log::info!("About to show dialog...");
    let dialog_result = macos.show_input_dialog(
        "Enter some text:",
        Some("Type anything you want:"),
        Some("default value"),
    ).await;

    log::info!("Dialog returned, processing result...");

    match dialog_result {
        Ok(Some(text)) => {
            let msg = format!("You entered: {}", text);
            log::info!("Showing success notification: {}", msg);
            show_notification(&msg);
            log::info!("Notification shown");
        }
        Ok(None) => {
            log::info!("User cancelled, showing cancel notification");
            show_notification("Input cancelled");
            log::info!("Cancel notification shown");
        }
        Err(e) => {
            log::error!("Dialog error: {}", e);
            return Err(e);
        }
    }

    log::info!("=== test_input_dialog: END ===");
    Ok(())
}

// Note: dump_app_menus and test_menu_click removed - menu functionality
// will be added back via macos.click_menu_item() when needed
