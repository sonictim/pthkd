//! Cached menu structures for macOS applications

use crate::menu_item::MenuItem;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    /// Cache of menu structures keyed by app name
    static ref MENU_CACHE: RwLock<HashMap<String, Vec<MenuItem>>> = RwLock::new(HashMap::new());
}

/// Get menus for an app (from cache or fetch from Swift)
pub fn get_menus(app_name: &str, refresh: bool) -> Result<Vec<MenuItem>> {
    // Check cache first (unless refresh requested)
    if !refresh {
        let cache = MENU_CACHE.read().unwrap();
        if let Some(menus) = cache.get(app_name) {
            log::debug!("Menu cache hit for '{}'", app_name);
            return Ok(menus.clone());
        }
    }

    // Fetch from Swift
    log::debug!("Fetching menus for '{}' from Swift", app_name);
    let json = crate::swift_bridge::get_app_menus(app_name)?;
    let menus = MenuItem::from_json(&json)?;

    // Update cache
    let mut cache = MENU_CACHE.write().unwrap();
    cache.insert(app_name.to_string(), menus.clone());

    Ok(menus)
}

/// Find a menu item by path
pub fn find_menu_item<'a>(menus: &'a [MenuItem], path: &[&str]) -> Option<&'a MenuItem> {
    MenuItem::find_by_path(menus, path)
}

/// Execute a menu item by path
pub fn execute_menu(app_name: &str, path: &[&str]) -> Result<()> {
    log::debug!(
        "execute_menu called for '{}' with path: {:?}",
        app_name,
        path
    );

    // Get menus (from cache if available)
    let menus = get_menus(app_name, false)?;
    log::debug!("Got {} top-level menus", menus.len());

    // Find the menu item
    let menu_item = find_menu_item(&menus, path).ok_or_else(|| {
        log::error!("Menu item not found! Searched for: {:?}", path);
        log::debug!(
            "Available menus: {:?}",
            menus.iter().map(|m| &m.title).collect::<Vec<_>>()
        );
        anyhow::anyhow!("Menu item not found: {:?}", path)
    })?;

    log::debug!(
        "Found menu item: '{}', has shortcut: {}",
        menu_item.title,
        menu_item.cmd_char.is_some()
    );
    log::debug!(
        "Found menu item: '{}', Full item: {:?}",
        menu_item.title,
        menu_item
    );

    // Execute it
    menu_item.execute(app_name)
}

/// Refresh menu cache for an app
pub fn refresh_menus(app_name: &str) -> Result<Vec<MenuItem>> {
    get_menus(app_name, true)
}

/// Clear the entire menu cache
pub fn clear_cache() {
    let mut cache = MENU_CACHE.write().unwrap();
    cache.clear();
    log::info!("Menu cache cleared");
}

/// Clear cache for a specific app
pub fn clear_app_cache(app_name: &str) {
    let mut cache = MENU_CACHE.write().unwrap();
    cache.remove(app_name);
    log::info!("Menu cache cleared for '{}'", app_name);
}

/// Get cache statistics (for debugging)
pub fn cache_stats() -> HashMap<String, usize> {
    let cache = MENU_CACHE.read().unwrap();
    cache
        .iter()
        .map(|(app, menus)| (app.clone(), MenuItem::flatten(menus).len()))
        .collect()
}
