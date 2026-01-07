use super::client::*;
use super::*;
use crate::actions_async;
use crate::hotkey::HotkeyCounter;
use crate::params::Params;
use anyhow::Result;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

actions_async!("pt", plugins, {
    audiosuite,
    multitap_selector,
    send_receive_rx,
});
// ============================================================================
// Command Implementations
// ============================================================================

pub async fn audiosuite(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let plugin = params.get_string("plugin", "");
    let button = params.get_string("button", "");
    let close = params.get_bool("close", false);
    let save = params.get_bool("save", true);
    call_plugin(&plugin, &button, close).await?;
    if save {
        pt.save_session().await?;
    }
    Ok(())
}
pub async fn send_receive_rx(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let version = params.get_int("version", 11);
    let plugin = format!("RX {} Connect", version);
    let rx_app = format!("RX {}", version);

    let app = crate::macos::app_info::get_current_app()?;
    if app == "Pro Tools" {
        // Send to RX for analysis
        call_plugin(&plugin, "Analyze", false).await?;
    } else if crate::soft_match(&app, &rx_app) {
        // Send back to Pro Tools - Cmd+Enter returns to DAW
        keystroke(&["cmd", "enter"]).await?;
        let mut windows = 2;
        // while windows == 1 {
        //     let app_windows = crate::macos::ui_elements::get_window_titles(&app)?;
        //     windows = app_windows
        //         .into_iter()
        //         .filter(|w| w == "Pro Tools 1")
        //         .count();
        // }
        // println!("Multiple windows detected");
        while windows > 1 {
            std::thread::sleep(std::time::Duration::from_millis(100)); // Wait 50ms
            let app_windows = crate::macos::ui_elements::get_window_titles(&app)?;
            windows = app_windows
                .into_iter()
                .filter(|w| w == "Pro Tools 1")
                .count();
        }
        println!("one window detected");
        crate::macos::app_info::focus_app("Pro Tools", "", true, false, 50).ok();
        crate::macos::ui_elements::wait_for_window_focused("Pro Tools", &plugin, 50).ok();
        // Focus Pro Tools and wait for confirmation (switch but don't launch)

        // Now render the changes back
        call_plugin(&plugin, "Render", false).await?;
    }

    Ok(())
}
pub async fn multitap_selector(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let plugins = params.get_string_vec("plugins");
    let button = params.get_string("button", "");
    let close = params.get_bool("close", false);
    let timeout_ms = params.get_timeout_ms("timeout", 500);

    plugin_selector(&plugins, button, close, timeout_ms).await?;

    // if save {
    //     pt.save_session().await?;
    // }
    Ok(())
}

/// Find the category for a plugin in the AudioSuite menu using the menu cache
fn find_plugin_category(plugin_name: &str) -> Result<String> {
    // Get menus from cache (menu_cache handles caching internally)
    let menus = crate::macos::menu_cache::get_menus("Pro Tools", false)?;

    // Find AudioSuite menu
    let audiosuite_menu = menus
        .iter()
        .find(|m| m.title == "AudioSuite")
        .ok_or_else(|| anyhow::anyhow!("AudioSuite menu not found"))?;

    // Search for plugin in the menu structure
    // Structure is: AudioSuite -> (container) -> Category -> (container) -> Plugin
    if let Some(children) = &audiosuite_menu.children {
        for middleman in children {
            if let Some(categories) = &middleman.children {
                for category in categories {
                    if let Some(middleman2_items) = &category.children {
                        for middleman2 in middleman2_items {
                            if let Some(plugins) = &middleman2.children {
                                for plugin in plugins {
                                    if crate::soft_match(&plugin.title, plugin_name) {
                                        return Ok(category.title.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    anyhow::bail!("Plugin '{}' not found in AudioSuite menu", plugin_name)
}

async fn activate_plugin(plugin_name: &str) -> Result<()> {
    let window = format!("AudioSuite: {}", plugin_name);

    // Check if already open
    if crate::swift_bridge::window_exists("Pro Tools", &window)? {
        println!("Plugin '{}' window already open", plugin_name);
        return Ok(());
    }

    // Find the category using menu cache
    let category = find_plugin_category(plugin_name)?;

    // Open it
    crate::macos::menu_cache::execute_menu("Pro Tools", &["AudioSuite", &category, plugin_name])?;

    // Wait for window
    if !crate::swift_bridge::wait_for_window(
        "Pro Tools",
        &window,
        crate::swift_bridge::WindowCondition::Exists,
        5000,
    )? {
        anyhow::bail!("Window '{}' did not appear within timeout", window);
    }

    Ok(())
}

pub async fn call_plugin(plugin: &str, button: &str, close: bool) -> Result<()> {
    if !plugin.is_empty() {
        activate_plugin(plugin).await?;
    }
    if !button.is_empty() {
        let window = format!("AudioSuite: {}", plugin);
        click_button(&window, button).await?;
    }
    if close {
        let window = format!("AudioSuite: {}", plugin);
        crate::swift_bridge::close_window("Pro Tools", &window, Some(10000))?;
    }

    Ok(())
}
lazy_static! {
    static ref PLUGIN_COUNTER: Arc<Mutex<HotkeyCounter>> =
        Arc::new(Mutex::new(HotkeyCounter::new()));
}
pub async fn plugin_selector(
    plugins: &[String],
    button: String,
    close: bool,
    timeout_ms: u64,
) -> Result<()> {
    let plugins = plugins.to_vec(); // Clone for closure
    let mut counter = PLUGIN_COUNTER.lock().unwrap();
    log::info!("plugin_selector called with {} plugins", plugins.len());

    // Move button, plugins, and close directly into the closure
    counter.press(timeout_ms, plugins.len() as u32, move |idx| async move {
        log::info!(
            "Callback executing with idx={}, plugins.len()={}",
            idx,
            plugins.len()
        );
        if let Some(plugin) = plugins.get(idx as usize) {
            log::info!("Opening plugin: {} with button: '{}'", plugin, button);
            call_plugin(plugin, &button, close).await.ok();
        } else {
            log::error!("No plugin found at index {}", idx);
        }
    });
    Ok(())
}
