use super::client::*;
use super::*;
use crate::actions_async;
use crate::hotkey::HotkeyCounter;
use crate::params::Params;
use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
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
        let _ = crate::macos::app_info::focus_app("Pro Tools", "", true, false, 50);
        let _ = crate::macos::ui_elements::wait_for_window_focused("Pro Tools", &plugin, 50);
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

    Ok(())
}

lazy_static! {
    static ref PLUGIN_MAP: Arc<Mutex<Option<HashMap<String, String>>>> = Arc::new(Mutex::new(None));
}
async fn get_audiosuite_map() -> Result<HashMap<String, String>> {
    let menu_bar = crate::macos::menu::get_app_menus("Pro Tools")?;
    let audiosuite_menu = menu_bar
        .menus
        .iter()
        .find(|m| m.title == "AudioSuite")
        .ok_or_else(|| anyhow::anyhow!("AudioSuite menu not found"))?;
    let mut map = HashMap::new();

    for middleman in &audiosuite_menu.children {
        for category in &middleman.children {
            for middleman2 in &category.children {
                for plugin in &middleman2.children {
                    if !map.contains_key(&plugin.title) {
                        map.insert(plugin.title.clone(), category.title.clone());
                    }
                }
            }
        }
    }
    Ok(map)
}

fn plugin_map_soft_search(key: &str, map: &HashMap<String, String>) -> Option<String> {
    if map.contains_key(key) {
        return map.get(key).cloned();
    }
    for k in map.keys() {
        if crate::soft_match(k, key) {
            return map.get(k).cloned();
        }
    }
    None
}

async fn activate_plugin(plugin_name: &str) -> Result<()> {
    // Check if we need to build the map
    let needs_build = {
        let map = PLUGIN_MAP.lock().unwrap();
        map.is_none()
    }; // lock dropped here

    if needs_build {
        let new_map = get_audiosuite_map().await?;
        let mut map = PLUGIN_MAP.lock().unwrap();
        if map.is_none() {
            // double-check in case another thread built it
            *map = Some(new_map);
        }
    }

    // Now get the category we need
    let category = {
        let map = PLUGIN_MAP.lock().unwrap();
        let Some(ref map_data) = *map else {
            anyhow::bail!("Failed to load plugin map");
        };
        plugin_map_soft_search(plugin_name, map_data)
    }; // lock dropped here

    let Some(category) = category else {
        anyhow::bail!("Plugin '{}' not found in AudioSuite menu", plugin_name);
    };

    let window = format!("AudioSuite: {}", plugin_name);

    // Check if already open
    if crate::macos::ui_elements::window_exists("Pro Tools", &window)? {
        println!("Plugin '{}' window already open", plugin_name);
        return Ok(());
    }

    // Open it
    call_menu(&["AudioSuite", &category, plugin_name]).await?;

    // Wait for window
    crate::macos::ui_elements::wait_for_window_exists("Pro Tools", &window, 5000)?;

    Ok(())
}

pub async fn call_plugin(plugin: &str, button: &str, close: bool) -> Result<()> {
    if !plugin.is_empty() {
        activate_plugin(&plugin).await?;
    }
    if !button.is_empty() {
        let window = format!("AudioSuite: {}", plugin);
        click_button(&window, &button).await?;
    }
    if close {
        let window = format!("AudioSuite: {}", plugin);
        crate::macos::ui_elements::close_window_with_retry("Pro Tools", &window, 10000)?;
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
    let close = Box::new(close);
    counter.press(timeout_ms, plugins.len() as u32, |idx| async move {
        if let Some(plugin) = plugins.get(idx as usize) {
            let _ = call_plugin(plugin, &button, *close).await;
        }
    });

    Ok(())
}
