//! MenuItem struct for macOS menu items

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub title: String,
    pub path: Vec<String>,
    pub enabled: bool,
    pub checked: bool,
    #[serde(rename = "cmdChar")]
    pub cmd_char: Option<String>,
    #[serde(rename = "cmdModifiers")]
    pub cmd_modifiers: Option<i32>,
    pub children: Option<Vec<MenuItem>>,
}

impl MenuItem {
    /// Create a MenuItem from JSON returned by Swift
    pub fn from_json(json: &str) -> Result<Vec<MenuItem>> {
        #[derive(Deserialize)]
        struct MenuResponse {
            app: String,
            menus: Vec<MenuItem>,
        }

        let response: MenuResponse = serde_json::from_str(json)?;
        Ok(response.menus)
    }

    /// Find a menu item by path (e.g., &["File", "Save"])
    /// Uses soft_match for flexible matching (case-insensitive, whitespace-insensitive)
    pub fn find_by_path<'a>(menus: &'a [MenuItem], path: &[&str]) -> Option<&'a MenuItem> {
        if path.is_empty() {
            return None;
        }

        log::debug!(
            "find_by_path: searching for '{}' in {} menus",
            path[0],
            menus.len()
        );

        // Search top level using soft_match
        for menu in menus {
            if crate::soft_match(&menu.title, path[0]) {
                log::debug!("Found '{}' menu (matched with '{}')", menu.title, path[0]);
                if path.len() == 1 {
                    return Some(menu);
                }
                // Search children
                if let Some(ref children) = menu.children {
                    log::debug!(
                        "'{}' has {} children, searching for '{}'",
                        menu.title,
                        children.len(),
                        path[1]
                    );
                    // Log first few children for debugging
                    if !children.is_empty() {
                        let child_titles: Vec<&str> =
                            children.iter().take(10).map(|c| c.title.as_str()).collect();
                        log::debug!("First {} children: {:?}", child_titles.len(), child_titles);
                    }
                    return Self::find_by_path(children, &path[1..]);
                } else {
                    log::warn!("'{}' menu has NO children!", menu.title);
                }
            }
        }

        log::debug!("'{}' not found in current level", path[0]);
        None
    }

    /// Execute this menu item via AXPress (direct execution, no UI interaction)
    pub fn execute(&self, app_name: &str) -> Result<()> {
        log::debug!("Executing '{}' via AXPress", self.title);
        let mut full_path = self.path.clone();
        full_path.push(self.title.clone());

        let path_refs: Vec<&str> = full_path.iter().map(|s| s.as_str()).collect();
        crate::swift_bridge::menu_click(app_name, &path_refs)
    }

    /// Get all menu items as a flat list (useful for searching)
    pub fn flatten(menus: &[MenuItem]) -> Vec<MenuItem> {
        let mut result = Vec::new();
        for menu in menus {
            result.push(menu.clone());
            if let Some(ref children) = menu.children {
                result.extend(Self::flatten(children));
            }
        }
        result
    }

    /// Pretty print menu structure for debugging
    pub fn print_tree(menus: &[MenuItem], indent: usize) {
        for menu in menus {
            let prefix = "  ".repeat(indent);
            let status = if !menu.enabled {
                " [DISABLED]"
            } else if menu.checked {
                " [✓]"
            } else {
                ""
            };

            let shortcut = if let Some(ref cmd) = menu.cmd_char {
                format!(" (⌘{})", cmd)
            } else {
                String::new()
            };

            println!("{}{}{}{}", prefix, menu.title, status, shortcut);

            if let Some(ref children) = menu.children {
                Self::print_tree(children, indent + 1);
            }
        }
    }
}
