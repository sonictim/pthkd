use super::client::*;
use super::ptsl;
use crate::hotkey::HotkeyCounter;
use crate::params::Params;
use anyhow::Result;
use ptsl::CommandId;

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

// ============================================================================
// Command Implementations
// ============================================================================

async fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    std::thread::sleep(std::time::Duration::from_millis(50)); // Wait 50ms
    Ok(())
}
async fn call_menu(menu: &[&str]) -> Result<()> {
    crate::macos::menu::run_menu_item("Pro Tools", menu)?;
    std::thread::sleep(std::time::Duration::from_millis(10)); // Wait 50ms
    Ok(())
}
async fn button(window: &str, button: &str) -> Result<()> {
    crate::macos::ui_elements::click_button("Pro Tools", window, button)?;
    std::thread::sleep(std::time::Duration::from_millis(20)); // Wait 50ms
    Ok(())
}

pub async fn solo_clear(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_soloed = attributes["is_soloed"].as_bool().unwrap_or(false);

        if is_soloed {
            solos.push(name.to_string());
        }
    }
    pt.solo_tracks(solos, false).await?;

    Ok(())
}

pub async fn solo_selected_tracks(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();
    let mut unsolos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_selected_str = attributes["is_selected"].as_str().unwrap_or("None");
        let is_selected = is_selected_str == "SetExplicitly";
        let is_soloed = attributes["is_soloed"].as_bool().unwrap_or(false);

        if is_soloed != is_selected {
            if is_selected {
                solos.push(name.to_string());
            } else {
                unsolos.push(name.to_string());
            }
        }
    }
    pt.solo_tracks(solos, true).await?;
    pt.solo_tracks(unsolos, false).await?;

    Ok(())
}

pub async fn add_selected_to_solos(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_selected_str = attributes["is_selected"].as_str().unwrap_or("None");
        let is_selected = is_selected_str != "None";

        if is_selected {
            solos.push(name.to_string());
        }
    }
    pt.solo_tracks(solos, true).await?;

    Ok(())
}
pub async fn remove_selected_from_solos(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_selected_str = attributes["is_selected"].as_str().unwrap_or("None");
        let is_selected = is_selected_str != "None";

        if is_selected {
            solos.push(name.to_string());
        }
    }
    pt.solo_tracks(solos, false).await?;

    Ok(())
}

/// Wrapper for crossfade with default preset (for use with pt_actions macro)
pub async fn crossfade(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    crossfade_and_clear_automation(pt, "TF Default", _params).await
}

pub async fn crossfade_and_clear_automation(
    pt: &mut ProtoolsSession,
    preset: &str,
    _params: &Params,
) -> Result<()> {
    let result = pt
        .cmd::<_, serde_json::Value>(
            CommandId::CreateFadesBasedOnPreset,
            ptsl::CreateFadesBasedOnPresetRequestBody {
                fade_preset_name: preset.to_string(),
                auto_adjust_bounds: true,
            },
        )
        .await;

    if result.is_err() {
        pt.cmd::<_, serde_json::Value>(
            CommandId::CreateFadesBasedOnPreset,
            ptsl::CreateFadesBasedOnPresetRequestBody {
                fade_preset_name: String::new(), // Last used
                auto_adjust_bounds: true,
            },
        )
        .await?;
    }
    let mut sel = PtSelectionSamples::new(pt).await?;
    let c = sel.get_io();
    sel.set_io(pt, c.1, c.1).await?;
    call_menu(&["Edit", "Automation", "Write to All Enabled"]).await?;
    // keystroke(&["cmd", "option", "slash"]).await?;
    sel.set_io(pt, c.0, c.0).await?;
    call_menu(&["Edit", "Automation", "Write to All Enabled"]).await?;
    // keystroke(&["cmd", "option", "slash"]).await?;
    sel.set_io(pt, c.0 + 100, c.1 - 100).await?;

    let _: serde_json::Value = pt
        .cmd(
            CommandId::ClearSpecial,
            ptsl::ClearSpecialRequestBody {
                automation_data_option: ptsl::AutomationDataOptions::AllAutomation.into(),
            },
        )
        .await?;

    sel.set_io(pt, c.0 - 48000, c.1 + 48000).await?;
    call_menu(&["Edit", "Automation", "Thin All"]).await?;
    // keystroke(&["cmd", "option", "control", "t"]).await?;
    sel.set_io(pt, c.0, c.1).await?;
    Ok(())
}

pub async fn conform_delete(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Conform Delete");
    let mut flag = false;
    let original_mode = pt.get_edit_mode().await?;
    pt.set_edit_mode("EMO_Shuffle").await?;

    if pt.get_edit_mode().await? != "EMO_Shuffle" {
        keystroke(&["cmd", "f1"]).await?;
        // std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
        pt.set_edit_mode("EMO_Shuffle").await?;
        flag = true;
    }
    let _: serde_json::Value = pt.cmd(CommandId::Clear, serde_json::json!({})).await?;
    pt.set_edit_mode(&original_mode).await?;
    if flag {
        keystroke(&["cmd", "f1"]).await?;
    }
    Ok(())
}
pub async fn conform_insert(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Conform Insert");
    let mut flag = false;
    let original_mode = pt.get_edit_mode().await?;
    pt.set_edit_mode("EMO_Shuffle").await?;

    if pt.get_edit_mode().await? != "EMO_Shuffle" {
        keystroke(&["cmd", "f1"]).await?;
        // std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
        pt.set_edit_mode("EMO_Shuffle").await?;
        flag = true;
    }
    call_menu(&["Edit", "Insert Silence"]).await?;
    // keystroke(&["cmd", "shift", "e"]).await?;
    // std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
    pt.set_edit_mode(&original_mode).await?;
    if flag {
        keystroke(&["cmd", "f1"]).await?;
    }
    Ok(())
}
pub async fn get_selection_samples(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    let mut selection = PtSelectionSamples::new(pt).await?;
    selection.slide(pt, 48000).await?;
    let (st, et) = selection.get_io();
    pt.edit_marker(
        1,
        "Tim's Cool Marker",
        st,
        et,
        MarkerLocation::NamedRuler,
        "Markers 5",
    )
    .await?;
    let markers = pt.get_all_markers().await.unwrap_or(Vec::new());
    println!("Marker List: {:?}", markers);
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    println!("Marker Ruler Names: {:?}", rulers);
    Ok(())
}

/// Navigate to a marker with parameterized ruler name and direction
///
/// Parameters:
/// - `reverse`: boolean - true for previous marker, false for next marker (default: false)
/// - `ruler`: string - name of the marker ruler to use, empty string for all markers (default: "")
pub async fn go_to_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let reverse = params.get_bool("reverse", false);
    let ruler = params.get_string("ruler", "");
    pt.go_to_next_marker(&ruler, reverse).await?;
    keystroke(&["left"]).await?;
    Ok(())
}
pub async fn toggle_edit_tool(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    let tool = pt.get_edit_tool().await?;
    if tool != "ET_Selector" {
        pt.set_edit_tool("ET_Selector").await?;
    } else {
        pt.set_edit_tool("ET_GrabberTime").await?;
    }
    Ok(())
}
pub async fn spot_to_protools_from_soundminer(
    _pt: &mut ProtoolsSession,
    _params: &Params,
) -> Result<()> {
    println!("Sending to Protools Session");
    crate::macos::menu::run_menu_item("Soundminer_Intel", &["Transfer", "Spot to DAW"])?;
    Ok(())
}

async fn plugin_render(menu: &str, plugin: &str, close: bool) -> Result<()> {
    let window = format!("AudioSuite: {}", plugin);
    if !crate::macos::ui_elements::window_exists("Pro Tools", &window)? {
        call_menu(&["AudioSuite", menu, plugin]).await?;
    }
    crate::macos::ui_elements::wait_for_window("Pro Tools", &window, 5000)?;
    button(&window, "Render").await?;
    if close {
        crate::macos::ui_elements::close_window_with_retry(
            "Pro Tools",
            &window,  // Fixed: was hardcoded to "AudioSuite: Reverse"
            10000,
        )?;
    }
    Ok(())
}
async fn plugin_analyze(menu: &str, plugin: &str) -> Result<()> {
    let window = format!("AudioSuite: {}", plugin);
    if !crate::macos::ui_elements::window_exists("Pro Tools", &window)? {
        call_menu(&["AudioSuite", menu, plugin]).await?;
    }
    crate::macos::ui_elements::wait_for_window("Pro Tools", &window, 5000)?;
    button(&window, "Analyze").await?;
    Ok(())
}

pub async fn reverse_selection(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let close = params.get_bool("close", true);
    plugin_render("Other", "Reverse", close).await?;
    Ok(())
}
pub async fn preview_audiosuite(_pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    button("AudioSuite", "Preview Processing").await?;
    Ok(())
}
pub async fn send_receive_rx(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let version = params.get_int("version", 11);
    let plugin = format!("RX {} Connect", version);
    let rx_app = format!("RX {}", version);

    let app = crate::macos::app_info::get_current_app()?;
    if app == "Pro Tools" {
        // Send to RX for analysis
        plugin_analyze("Noise Reduction", &plugin).await?;
    } else if crate::soft_match(&app, &rx_app) {
        // Send back to Pro Tools - Cmd+Enter returns to DAW
        keystroke(&["cmd", "enter"]).await?;

        // Wait for Pro Tools to be focused (any window)
        crate::macos::ui_elements::wait_for_window_focused("Pro Tools", "", 10000)?;

        // Now render the changes back
        plugin_render("Noise Reduction", &plugin, false).await?;
    }

    Ok(())
}
lazy_static! {
    static ref PLUGIN_COUNTER: Arc<Mutex<HotkeyCounter>> =
        Arc::new(Mutex::new(HotkeyCounter::new()));
}

pub async fn open_plugin(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut counter = PLUGIN_COUNTER.lock().unwrap();
    let plugins = params.get_string_pairs("plugins");
    let timeout_ms = params.get_timeout_ms("timeout", 500);

    // Clone plugins to move into the async closure
    let plugins_clone = plugins.clone();

    // Pass timeout, max, and async callback - cycle through all plugins (0-based indexing)
    counter.press(timeout_ms, plugins.len() as u32, |count| async move {
        if let Some((manufacturer, name)) = plugins_clone.get(count as usize) {
            let menu_path = ["AudioSuite", manufacturer.as_str(), name.as_str()];
            if let Err(e) = call_menu(&menu_path).await {
                log::error!("Failed to open plugin {}/{}: {:#}", manufacturer, name, e);
            }
        }
    });

    Ok(())
}

lazy_static! {
    static ref PLUGIN_TYPE_COUNTER: Arc<Mutex<HotkeyCounter>> =
        Arc::new(Mutex::new(HotkeyCounter::new()));
}

pub async fn open_plugin_type(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut counter = PLUGIN_TYPE_COUNTER.lock().unwrap();
    let plugin_type = params.get_string("type", "");
    let plugins = params.get_string_vec("plugins");
    let timeout_ms = params.get_timeout_ms("timeout", 500);

    // Clone to move into the async closure
    let plugins_clone = plugins.clone();
    let plugin_type_clone = plugin_type.clone();

    // Pass timeout, max, and async callback - cycle through all plugins (0-based indexing)
    counter.press(timeout_ms, plugins.len() as u32, |count| async move {
        if let Some(name) = plugins_clone.get(count as usize) {
            let menu_path = ["AudioSuite", plugin_type_clone.as_str(), name.as_str()];
            if let Err(e) = call_menu(&menu_path).await {
                log::error!(
                    "Failed to open plugin {}/{}: {:#}",
                    plugin_type_clone,
                    name,
                    e
                );
            }
        }
    });

    Ok(())
}
