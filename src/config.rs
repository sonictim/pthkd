use crate::hotkey::{ChordPattern, Hotkey};
use crate::keycodes::key_name_to_codes;
use crate::params::Params;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// Embedded default configuration
const DEFAULT_CONFIG: &str = include_str!("../config.toml");

/// Helper type to deserialize either a single string or array of strings
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    fn into_vec(self) -> Vec<String> {
        match self {
            StringOrVec::Single(s) => vec![s],
            StringOrVec::Multiple(v) => v,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub hotkey: Vec<HotkeyConfig>,
}

#[derive(Debug, Deserialize)]
pub struct HotkeyConfig {
    pub keys: Vec<String>,
    pub action: String,
    #[serde(default)]
    pub params: HashMap<String, toml::Value>,
    #[serde(default)]
    pub trigger_on_release: bool,
    #[serde(default)]
    pub notify: bool,
    pub target_application: Option<StringOrVec>,
    pub app_window: Option<String>,
}

/// Load and parse the config file
/// If the config file doesn't exist, creates one from the embedded default
/// If the config file has TOML errors, falls back to the embedded default
pub fn load_config(path: &str) -> Result<Config> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => {
            log::info!("Loaded config from {}", path);
            contents
        }
        Err(_) => {
            log::warn!("Config file '{}' not found, creating from default", path);
            // Write the default config to disk
            fs::write(path, DEFAULT_CONFIG)
                .context("Failed to write default config file")?;
            log::info!("Created default config at {}", path);
            DEFAULT_CONFIG.to_string()
        }
    };

    // Try to parse the config
    let config = match toml::from_str::<Config>(&contents) {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to parse config file: {:#}", e);
            log::warn!("Falling back to embedded default configuration");
            toml::from_str(DEFAULT_CONFIG)
                .context("Failed to parse embedded default config (this should never happen)")?
        }
    };

    Ok(config)
}

/// Convert config hotkeys to runtime Hotkey structs
/// Skips any hotkeys that fail to parse instead of failing entirely
pub fn config_to_hotkeys(config: Config) -> Result<Vec<Hotkey>> {
    let mut hotkeys = Vec::new();
    let mut skipped_count = 0;

    for hk_config in config.hotkey {
        // Parse the chord pattern from the keys list
        let chord = match parse_chord(&hk_config.keys) {
            Ok(chord) => chord,
            Err(e) => {
                log::error!("Skipping hotkey with keys {:?}: {:#}", hk_config.keys, e);
                skipped_count += 1;
                continue;
            }
        };

        // Look up the action function (handles namespaces)
        let action = match get_action(&hk_config.action) {
            Some(action) => action,
            None => {
                log::error!("Skipping hotkey '{}': unknown action", hk_config.action);
                skipped_count += 1;
                continue;
            }
        };

        hotkeys.push(Hotkey {
            chord,
            action_name: hk_config.action.clone(),
            action,
            params: Params::new(hk_config.params),
            trigger_on_release: hk_config.trigger_on_release,
            notify: hk_config.notify,
            application: hk_config.target_application.map(|app| app.into_vec()),
            app_window: hk_config.app_window,
        });
    }

    if skipped_count > 0 {
        log::warn!("Skipped {} invalid hotkey(s)", skipped_count);
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
pub fn get_action(name: &str) -> Option<fn(&Params) -> anyhow::Result<()>> {
    // Check if action is namespaced (contains '.')
    if let Some((namespace, action_name)) = name.split_once('.') {
        match namespace {
            "pt" => crate::protools::get_action_registry()
                .get(action_name)
                .copied(),
            "os" => crate::macos::actions::get_action_registry()
                .get(action_name)
                .copied(),
            "sm" => crate::soundminer::actions::get_action_registry()
                .get(action_name)
                .copied(),
            _ => None,
        }
    } else {
        // Unnamespaced - try each registry in order: os, pt, sm
        crate::macos::actions::get_action_registry()
            .get(name)
            .copied()
            .or_else(|| {
                crate::protools::get_action_registry()
                    .get(name)
                    .copied()
            })
            .or_else(|| {
                crate::soundminer::actions::get_action_registry()
                    .get(name)
                    .copied()
            })
    }
}
