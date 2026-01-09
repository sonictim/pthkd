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

// ============================================================================
// Plugin Map Cache
// ============================================================================

lazy_static! {
    /// Cache mapping plugin names to (category, exact_name)
    static ref PLUGIN_MAP: Arc<Mutex<Option<HashMap<String, (String, String)>>>> =
        Arc::new(Mutex::new(None));
}

/// Build the plugin map by traversing the AudioSuite menu tree once
fn build_plugin_map() -> Result<HashMap<String, (String, String)>> {
    let menus = crate::macos::menu_cache::get_menus("Pro Tools", false)?;

    let audiosuite_menu = menus
        .iter()
        .find(|m| m.title == "AudioSuite")
        .ok_or_else(|| anyhow::anyhow!("AudioSuite menu not found"))?;

    let Some(ref children) = audiosuite_menu.children else {
        anyhow::bail!("AudioSuite menu has no children");
    };

    let mut map = HashMap::new();

    // Recursively collect all plugins
    fn collect_plugins(
        items: &[crate::menu_item::MenuItem],
        parent_category: &str,
        map: &mut HashMap<String, (String, String)>,
    ) {
        for item in items {
            // If we're at the top level (parent_category is empty), this item might be a category
            let category = if parent_category.is_empty() {
                &item.title
            } else {
                parent_category
            };

            // Add this item as a potential plugin
            // Use lowercase key for case-insensitive lookups
            let key = item.title.to_lowercase();
            map.entry(key)
                .or_insert_with(|| (category.to_string(), item.title.clone()));

            // Recursively process children
            if let Some(ref children) = item.children {
                collect_plugins(children, category, map);
            }
        }
    }

    collect_plugins(children, "", &mut map);

    Ok(map)
}

/// Get or build the plugin map
fn get_plugin_map() -> Result<HashMap<String, (String, String)>> {
    let mut cache = PLUGIN_MAP.lock().unwrap();

    if cache.is_none() {
        log::info!("Building AudioSuite plugin map cache...");
        let map = build_plugin_map()?;
        log::info!("Plugin map cached with {} entries", map.len());
        *cache = Some(map);
    }

    Ok(cache.as_ref().unwrap().clone())
}

/// Find the category for a plugin in the AudioSuite menu using cached HashMap
/// Returns (category_name, exact_plugin_name)
fn find_plugin_category(plugin_name: &str) -> Result<(String, String)> {
    let map = get_plugin_map()?;

    // Try exact match first (case-insensitive)
    let key = plugin_name.to_lowercase();
    if let Some(result) = map.get(&key) {
        return Ok(result.clone());
    }

    // Fall back to soft match (partial matching)
    for (map_key, value) in &map {
        if crate::soft_match(map_key, plugin_name) {
            return Ok(value.clone());
        }
    }

    anyhow::bail!("Plugin '{}' not found in AudioSuite menu", plugin_name)
}

async fn activate_plugin_internal(category: &str, exact_name: &str) -> Result<()> {
    let window = format!("AudioSuite: {}", exact_name);

    // Check if already open
    if crate::swift_bridge::window_exists("Pro Tools", &window)? {
        println!("Plugin '{}' window already open", exact_name);
        return Ok(());
    }

    // Open it - build menu path based on whether there's a category
    if category.is_empty() {
        // No category - plugin is at root level
        crate::swift_bridge::menu_click("Pro Tools", &["AudioSuite", exact_name])?;
    } else {
        // Has category
        crate::swift_bridge::menu_click("Pro Tools", &["AudioSuite", category, exact_name])?;
    }

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

async fn activate_plugin(plugin_name: &str) -> Result<()> {
    let (category, exact_name) = find_plugin_category(plugin_name)?;
    activate_plugin_internal(&category, &exact_name).await
}

pub async fn call_plugin(plugin: &str, button: &str, close: bool) -> Result<()> {
    // Get the exact plugin name from the menu (search once)
    let exact_name = if !plugin.is_empty() {
        let (category, exact) = find_plugin_category(plugin)?;
        activate_plugin_internal(&category, &exact).await?;
        exact
    } else {
        plugin.to_string()
    };

    if !button.is_empty() {
        let window = format!("AudioSuite: {}", exact_name);
        click_button(&window, button).await?;
    }
    if close {
        let window = format!("AudioSuite: {}", exact_name);
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
