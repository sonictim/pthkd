//! macOS system command implementations

use super::{MacOSSession, app_info, keystroke};
use crate::prelude::*;
use std::process::Command;

// ============================================================================
// Command Implementations
// ============================================================================

pub fn test_notification(_params: &Params) -> R<()> {
    MacOSSession::global().show_notification("CMD+Shift+K pressed!");
    Ok(())
}

pub fn execute_menu_item(params: &Params) -> R<()> {
    let app_name = params.get_string("app", "");
    let menu_path = params.get_str_vec("menu");

    if menu_path.is_empty() {
        anyhow::bail!("Menu path is required (e.g., menu = [\"File\", \"Save\"])");
    }

    log::info!("Testing Swift menu click: {} -> {:?}", app_name, menu_path);

    // Convert Vec<String> to Vec<&str>
    let path_refs: Vec<&str> = menu_path.iter().map(|s| &s[..]).collect();

    // Use the menu cache to execute
    match OS::menu_click(&app_name, &path_refs) {
        Ok(_) => {
            log::info!("✅ Menu click succeeded!");
            Ok(())
        }
        Err(e) => {
            log::error!("❌ Menu click failed: {}", e);
            Err(e)
        }
    }
}
pub fn test_window(_params: &Params) -> R<()> {
    log::info!("=== test_window: Dispatching to main queue ===");

    unsafe {
        super::dispatch_to_main_queue(|| {
            log::info!("Main queue: Showing message dialog...");

            if let Err(e) = MacOSSession::global().show_message_dialog("Hello World 2") {
                log::error!("Message dialog error: {}", e);
                return;
            }
            log::info!("Main queue: Message dialog closed");

            log::info!("Main queue: Showing text window...");
            if let Err(e) = MacOSSession::global().show_text_window("Hello World") {
                log::error!("Text window error: {}", e);
                return;
            }
            log::info!("Main queue: Text window shown");
        });
    }

    log::info!("Event callback returning immediately");
    Ok(())
}

pub fn test_modal_window(_params: &Params) -> R<()> {
    log::info!("=== test_modal_window: Dispatching to main queue ===");

    unsafe {
        super::dispatch_to_main_queue(|| {
            log::info!("Main queue: Showing modal dialog...");

            if let Err(e) = MacOSSession::global().show_message_dialog("Modal Dialog Test") {
                log::error!("Modal dialog error: {}", e);
            } else {
                log::info!("Main queue: Modal dialog closed");
            }
        });
    }

    log::info!("Event callback returning immediately");
    Ok(())
}

pub fn test_text_window(_params: &Params) -> R<()> {
    log::info!("=== test_text_window: Dispatching to main queue ===");

    unsafe {
        super::dispatch_to_main_queue(|| {
            if let Err(e) = MacOSSession::global().show_text_window("Text Window Test") {
                log::error!("Text window error: {}", e);
            } else {
                log::info!("Main queue: Text window shown");
            }
        });
    }

    log::info!("Event callback returning immediately");
    Ok(())
}

pub fn test_keystroke(_params: &Params) -> R<()> {
    log::info!("Testing global keystroke - sending CMD+F1");
    keystroke(&["cmd", "f1"])?;
    log::info!("Keystroke sent successfully");
    Ok(())
}

pub fn test_app_info(_params: &Params) -> R<()> {
    use std::time::Instant;
    let mut log = crate::MessageLog::default();
    log.append("=== App Focus Information ===");

    // Benchmark get_current_app()
    let start = Instant::now();
    for _ in 0..1000 {
        app_info::get_current_app().ok();
    }
    let elapsed = start.elapsed();
    let msg = format!(
        "⏱️  get_current_app() benchmark: {:?} per call (1000 calls in {:?})",
        elapsed / 1000,
        elapsed
    );
    log.append(&msg);

    // Get current app (no permissions needed)
    match app_info::get_current_app() {
        Ok(app_name) => {
            log.append(&format!("Current App: {}", app_name));
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
        Ok(title) => log.append(&format!("Window Title: {}", title)),
        Err(e) => log::error!("Failed to get window: {}", e),
    }

    // Check if in text field (requires permissions)
    match app_info::is_in_text_field() {
        Ok(is_text) => {
            if is_text {
                log.append("Text Field: ✅ Yes (cursor is in a text entry field)");
            } else {
                log.append("Text Field: ❌ No (not in a text field)");
            }
        }
        Err(e) => log::error!("Failed to check text field: {}", e),
    }
    log.display()
}

pub fn reload_config(_params: &Params) -> R<()> {
    use crate::config::{config_to_hotkeys, load_config};
    use crate::hotkey::HOTKEYS;
    use anyhow::{Context, bail};

    log::info!("⚠️  reload_config STARTED");
    log::info!("Reloading config from config.toml...");

    // Load and parse config
    log::info!("⚠️  About to call load_config");
    let config = load_config("config.toml").context("Failed to load config.toml")?;
    log::info!("⚠️  load_config completed");

    // Convert to hotkeys
    log::info!("⚠️  About to call config_to_hotkeys");
    let hotkeys = config_to_hotkeys(config).context("Failed to parse config")?;
    log::info!("⚠️  config_to_hotkeys completed");

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

pub fn dump_app_menus(_params: &Params) -> R<()> {
    use anyhow::Context;

    // Get menu structure for Pro Tools
    let app_name = "Pro Tools";
    log::info!("Getting menu structure for {}...", app_name);

    let json =
        OS::get_app_menus(app_name).context(format!("Failed to get menus for {}", app_name))?;

    let log = crate::MessageLog::new(&format!("Menu structure for {}:\n{}", app_name, json));
    log.display()
}

pub fn list_running_apps(_params: &Params) -> R<()> {
    use anyhow::Context;

    log::info!("Getting list of running applications...");

    let apps =
        app_info::get_all_running_applications().context("Failed to get running applications")?;

    log::info!("Running applications ({}):", apps.len());
    let mut log = crate::MessageLog::new(&format!("=== Running Applications ({}) ===", apps.len()));
    for app in &apps {
        log.append(&format!("  - {}", app));
    }
    log.display()
}

pub fn focus_protools(_params: &Params) -> R<()> {
    use anyhow::Context;

    log::info!("Focusing Pro Tools...");
    app_info::focus_application("Pro Tools").context("Failed to focus Pro Tools")?;
    log::info!("✅ Pro Tools focused successfully!");

    Ok(())
}
pub fn launch_application(params: &Params) -> R<()> {
    use anyhow::Context;
    let app = params.get_str("app", "");
    if !app.is_empty() {
        log::info!("Launching {app}...");
        app_info::launch_application(app).context("Failed to launch {app}")?;
        log::info!("✅ {app} launched successfully!");
    }
    Ok(())
}

pub fn list_window_buttons(params: &Params) -> R<()> {
    let current_app = crate::macos::app_info::get_current_app()
        .ok()
        .unwrap_or_default();
    let app_name = params.get_string("app", &current_app);
    let window_name = params.get_string("window", "");
    let debug = params.get_bool("debug", false);

    log::info!(
        "Listing buttons in window '{}' of app '{}'...",
        if window_name.is_empty() {
            "<focused>"
        } else {
            &window_name
        },
        app_name
    );

    match OS::get_window_buttons(&app_name, &window_name) {
        Ok(buttons) => {
            let mut log = crate::MessageLog::new("\n=== Buttons in window ===");
            log.append(&format!("App: {}", app_name));
            log.append(&format!(
                "Window: {}",
                if window_name.is_empty() {
                    "<focused>"
                } else {
                    &window_name
                }
            ));
            log.append(&format!("\nFound {} buttons:", buttons.len()));
            for (i, button) in buttons.iter().enumerate() {
                log.append(&format!("  {}. {}", i + 1, button));
            }
            log.display()
        }
        Err(e) => {
            let mut log = crate::MessageLog::new("\n❌ Error getting window buttons");
            log.append(&format!("App: {}", app_name));
            log.append(&format!(
                "Window: {}",
                if window_name.is_empty() {
                    "<focused>"
                } else {
                    &window_name
                }
            ));
            log.append(&format!("\nError: {}", e));

            // Add debug info if requested
            if debug {
                log.append("\n=== Debug Info ===");

                // Show current app
                if let Ok(current_app) = super::app_info::get_current_app() {
                    log.append(&format!("Current frontmost app: {}", current_app));
                }

                // Show running apps
                if let Ok(running_apps) = OS::get_running_apps() {
                    log.append(&format!("\nRunning apps matching '{}':", app_name));
                    for app in running_apps
                        .iter()
                        .filter(|a| crate::soft_match(a, &app_name))
                    {
                        log.append(&format!("  - {}", app));
                    }
                }

                // Show window titles
                if let Ok(titles) = OS::get_window_titles(&app_name) {
                    log.append(&format!(
                        "\nWindows for '{}' ({} total):",
                        app_name,
                        titles.len()
                    ));
                    for (i, title) in titles.iter().enumerate() {
                        log.append(&format!("  {}. {}", i + 1, title));
                    }
                }

                // Check accessibility permissions
                log.append(&format!(
                    "\nAccessibility permissions: {}",
                    if app_info::has_accessibility_permission() {
                        "✅ Granted"
                    } else {
                        "❌ Not granted"
                    }
                ));
            }

            log.append("\nPossible causes:");
            log.append("  1. Application is not running");
            log.append("  2. Window does not exist or is not focused");
            log.append("  3. Accessibility permissions not granted");
            log.append("     (System Settings > Privacy & Security > Accessibility)");
            log.append("\nTip: Add 'debug = true' to see more diagnostic info");
            log.display()
        }
    }
}

pub fn click_window_button(params: &Params) -> R<()> {
    use anyhow::Context;

    let app_name = params.get_string("app", "Pro Tools");
    let window_name = params.get_string("window", "");
    let button_name = params.get_string("button", "");

    if button_name.is_empty() {
        anyhow::bail!("button parameter is required");
    }

    log::info!(
        "Clicking button '{}' in window '{}' of app '{}'...",
        button_name,
        if window_name.is_empty() {
            "<focused>"
        } else {
            &window_name
        },
        app_name
    );

    OS::click_button(&app_name, &window_name, &button_name).context("Failed to click button")?;

    Ok(())
}

pub fn display_window_text(_params: &Params) -> R<()> {
    log::info!("Getting text from focused window...");

    // Get current app
    let app_name = super::app_info::get_current_app()?;
    let mut log = crate::MessageLog::default();
    log.append(&format!("Current app: {}", app_name));

    // Get text from focused window (empty string = focused window)
    match OS::get_window_text(&app_name, "") {
        Ok(text_elements) => {
            log.append(&format!(
                "\n=== Window Text ({} elements) ===",
                text_elements.len()
            ));
            for (i, text) in text_elements.iter().enumerate() {
                log.append(&format!("  {}. {}", i + 1, text));
            }
        }
        Err(e) => {
            log::error!("Failed to get window text: {}", e);
            println!("❌ Failed to get window text: {}", e);
        }
    }

    Ok(())
}

pub fn test_input_dialog(_params: &Params) -> R<()> {
    use super::input_dialog;

    log::info!("=== test_input_dialog: START ===");

    log::info!("About to show dialog...");
    let dialog_r = input_dialog::show_input_dialog(
        "Enter some text:",
        Some("Type anything you want:"),
        Some("default value"),
    );

    log::info!("Dialog returned, processing r...");

    match dialog_r {
        Ok(Some(text)) => {
            let msg = format!("You entered: {}", text);
            log::info!("Showing success notification: {}", msg);
            MacOSSession::global().show_notification(&msg);
            log::info!("Notification shown");
        }
        Ok(None) => {
            log::info!("User cancelled, showing cancel notification");
            MacOSSession::global().show_notification("Input cancelled");
            log::info!("Cancel notification shown");
        }
        Err(e) => {
            log::error!("Dialog error: {}", e);
            return Err(e);
        }
    }

    log::info!("=== test_input_dialog: END ===");

    // Check and recreate event tap if disabled by input dialog
    if let Err(e) = super::recreate_event_tap_if_needed() {
        log::error!("Failed to recreate event tap after input dialog: {}", e);
    }

    Ok(())
}

/// Wait for all keys to be released before proceeding
/// This is critical for Carbon hotkeys which fire on keydown while keys are still pressed
fn wait_for_all_keys_released(max_wait_ms: u64) -> R<()> {
    use crate::hotkey::KEY_STATE;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_millis(max_wait_ms);

    eprintln!(
        "Waiting for all keys to be released (timeout: {}ms)...",
        max_wait_ms
    );

    loop {
        if let Some(key_state) = KEY_STATE.get() {
            let state = key_state.lock().unwrap();
            let pressed_keys = state.get_pressed_keys();
            let num_pressed = pressed_keys.len();

            if num_pressed == 0 {
                eprintln!("✓ All keys released!");
                return Ok(());
            }

            if start.elapsed() > timeout {
                eprintln!(
                    "⚠️ Timeout waiting for keys to be released ({} keys still pressed)",
                    num_pressed
                );
                return Ok(()); // Proceed anyway rather than failing
            }

            // Log which keys are still pressed (for debugging)
            if start.elapsed().as_millis() % 100 == 0 {
                eprintln!("  Still waiting... ({} keys pressed)", num_pressed);
            }

            drop(state); // Release lock before sleeping
        }

        // Check every 10ms
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn rapid_pw(params: &Params) -> R<()> {
    let account = params.get_str("account", "rapid_pw");
    let set = params.get_bool("set", false);
    let r = if set {
        unsafe { MacOSSession::global().password_prompt(account) }
    } else if let Ok(pw) = super::keyring::password_get(account) {
        log::info!(
            "Pasting password from keychain for account: {} (length: {} chars)",
            account,
            pw.len()
        );

        // CRITICAL: Wait for user to release ALL keys before pasting
        // Carbon hotkeys fire on keydown, so modifier keys are still pressed
        // We need to wait until they're released or the paste will have wrong modifiers
        wait_for_all_keys_released(500)?;

        eprintln!("All keys released, pasting password...");

        // Use paste instead of typing - much faster and more reliable
        super::keystroke::paste_text_for_password(&pw)?;

        // Small delay to ensure paste completes before Enter
        std::thread::sleep(std::time::Duration::from_millis(100));

        log::info!("Sending Enter key");
        keystroke(&["return"])
    } else {
        log::warn!("Password not found in keychain for account: {}", account);
        unsafe { MacOSSession::global().password_prompt(account) }
    };

    // Check and recreate event tap if it was disabled by keychain dialog
    if let Err(e) = super::recreate_event_tap_if_needed() {
        log::error!(
            "Failed to recreate event tap after keychain operation: {}",
            e
        );
    }

    r
}

pub fn test_pw(params: &Params) -> R<()> {
    let account = "test_pw";
    let set = params.get_bool("set", false);
    println!("Running Test PW");
    let r = if set {
        println!("setting");
        super::keyring::password_set(account, "test")
    } else if let Ok(pw) = super::keyring::password_get(account) {
        println!("typing password: {}", pw);
        super::keystroke::type_text(&pw)?;
        keystroke(&["enter"])
    } else {
        println!("Password not found.  Setting");
        super::keyring::password_set(account, "test")
    };

    // Check and recreate event tap if it was disabled by keychain dialog
    if let Err(e) = super::recreate_event_tap_if_needed() {
        log::error!(
            "Failed to recreate event tap after keychain operation: {}",
            e
        );
    }

    r
}

pub fn list_window_titles(params: &Params) -> R<()> {
    let app_name = params.get_string("app", "");
    let mut log = crate::MessageLog::default();

    if app_name.is_empty() {
        // If no app specified, use current app
        match super::app_info::get_current_app() {
            Ok(current_app) => {
                log::info!("Getting window titles for current app: {}", current_app);

                match OS::get_window_titles(&current_app) {
                    Ok(titles) => {
                        log.append(&format!(
                            "\n=== Window Titles for '{}' ({} windows) ===",
                            current_app,
                            titles.len()
                        ));
                        for (i, title) in titles.iter().enumerate() {
                            log.append(&format!("  {}. {}", i + 1, title));
                        }
                    }
                    Err(e) => {
                        log.append(&format!("❌ Error getting window titles: {}", e));
                    }
                }
            }
            Err(e) => {
                log.append(&format!("❌ Error getting current app: {}", e));
            }
        }
    } else {
        log::info!("Getting window titles for app: {}", app_name);

        match OS::get_window_titles(&app_name) {
            Ok(titles) => {
                log.append(&format!(
                    "\n=== Window Titles for '{}' ({} windows) ===",
                    app_name,
                    titles.len()
                ));
                for (i, title) in titles.iter().enumerate() {
                    log.append(&format!("  {}. {}", i + 1, title));
                }
            }
            Err(e) => {
                log.append(&format!("❌ Error getting window titles: {}", e));
            }
        }
    }

    log.display()
}

pub fn shell_script(params: &Params) -> R<()> {
    let script = params.get_str("script_path", "");
    println!("script from params: {}", script);
    if script.is_empty() {
        return Err(anyhow::anyhow!("No Script Parameter Entered"));
    }
    match run_shell_script(script) {
        Ok(r) => {
            log::info!("Shell Script Successful: {}", r);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn run_shell_script(script_path: &str) -> R<String> {
    println!("running shell script: {}", script_path);
    let output = Command::new("sh")
        .arg("-c")
        .arg(script_path)
        .output()
        .context("Failed to execute shell script")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Script failed: {}", stderr)
    }
}
