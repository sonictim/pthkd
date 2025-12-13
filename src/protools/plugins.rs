use crate::hotkey::HotkeyCounter;
use anyhow::Result;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
// ============================================================================
// Command Implementations
// ============================================================================

async fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
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

async fn activate_plugin(macos: &mut crate::macos::MacOSSession, plugin_name: &str) -> Result<()> {
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

    // if macos.window_exists("Pro Tools", &window).await? {
    //     println!("Plugin '{}' window already open", plugin_name);
    //     return Ok(());
    // }

    macos
        .click_menu_item("Pro Tools", &["AudioSuite", &category, plugin_name])
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Wait for window to appear
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(5);
    while start.elapsed() < timeout {
        if macos
            .window_exists("Pro Tools", &window)
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    anyhow::bail!("Timeout waiting for plugin window '{}'", window)
}

pub async fn call_plugin(plugin: &str, button: &str, close: bool) -> Result<()> {
    let mut macos = crate::macos::MacOSSession::new()?;

    if !plugin.is_empty() {
        activate_plugin(&mut macos, plugin).await?;
    }
    if !button.is_empty() {
        let window = format!("AudioSuite: {}", plugin);
        macos.click_button("Pro Tools", &window, button).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }
    if close {
        let window = format!("AudioSuite: {}", plugin);
        tokio::time::sleep(tokio::time::Duration::from_millis(35)).await;
        macos.close_window("Pro Tools", &window).await?;
    }
    Ok(())
}
lazy_static! {
    static ref PLUGIN_COUNTER: Arc<Mutex<HotkeyCounter>> =
        Arc::new(Mutex::new(HotkeyCounter::new()));
}
pub async fn plugin_selector(plugins: &[String], timeout_ms: u64) -> Result<()> {
    let plugins = plugins.to_vec(); // Clone for closure

    let mut counter = PLUGIN_COUNTER.lock().unwrap();
    counter.press(timeout_ms, plugins.len() as u32, |idx| async move {
        if let Some(plugin) = plugins.get(idx as usize) {
            let _ = call_plugin(plugin, "", false).await;
        }
    });

    Ok(())
}
