use crate::hotkey::{ChordPattern, Hotkey};
use crate::keycodes::key_name_to_codes;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub hotkey: Vec<HotkeyConfig>,
}

#[derive(Debug, Deserialize)]
pub struct HotkeyConfig {
    pub keys: Vec<String>,
    pub action: String,
    #[serde(default)]
    pub trigger_on_release: bool,
    pub target_application: Option<String>,
    pub app_window: Option<String>,
}

/// Load and parse the config file
pub fn load_config(path: &str) -> Result<Config> {
    let contents = fs::read_to_string(path).context("Failed to read config file")?;
    let config: Config = toml::from_str(&contents).context("Failed to parse TOML config")?;
    Ok(config)
}

/// Convert config hotkeys to runtime Hotkey structs
pub fn config_to_hotkeys(config: Config) -> Result<Vec<Hotkey>> {
    let mut hotkeys = Vec::new();

    for hk_config in config.hotkey {
        // Parse the chord pattern from the keys list
        let chord = parse_chord(&hk_config.keys)
            .with_context(|| format!("Failed to parse keys: {:?}", hk_config.keys))?;

        // Look up the action function (handles namespaces)
        let action = get_action(&hk_config.action)
            .with_context(|| format!("Unknown action: {}", hk_config.action))?;

        hotkeys.push(Hotkey {
            chord,
            action_name: hk_config.action.clone(),
            action,
            trigger_on_release: hk_config.trigger_on_release,
            application: hk_config.target_application,
            app_window: hk_config.app_window,
        });
    }

    Ok(hotkeys)
}

/// Parse a list of key names into a ChordPattern
///
/// For simultaneous chords, each key name maps to one or more keycodes.
/// For example: ["cmd", "shift", "s"] becomes:
/// - "cmd" → [55, 54] (left or right CMD)
/// - "shift" → [56, 60] (left or right Shift)
/// - "s" → [1] (S key)
///
/// This creates a Simultaneous chord with key_groups: [[55,54], [56,60], [1]]
fn parse_chord(key_names: &[String]) -> Result<ChordPattern> {
    if key_names.is_empty() {
        bail!("Chord cannot be empty");
    }

    let mut key_groups = Vec::new();

    for key_name in key_names {
        let codes = key_name_to_codes(key_name)
            .with_context(|| format!("Unknown key name: {}", key_name))?;

        key_groups.push(codes);
    }

    Ok(ChordPattern::Simultaneous { key_groups })
}

/// Look up an action by name, handling namespaces
pub fn get_action(name: &str) -> Option<fn()> {
    // Check if action is namespaced (contains '.')
    if let Some((namespace, action_name)) = name.split_once('.') {
        match namespace {
            "pt" => crate::protools::actions::get_action_registry()
                .get(action_name)
                .copied(),
            // Future: "logic" => crate::logic::actions::get_action_registry()...
            _ => None,
        }
    } else {
        // Unnamespaced - look in main actions
        crate::actions::get_action_registry().get(name).copied()
    }
}
