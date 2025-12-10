use super::client::*;
use super::ptsl;
use crate::hotkey::HotkeyCounter;
use crate::params::Params;
use anyhow::{Result, bail};
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
async fn call_menu(macos: &mut crate::macos::MacOSSession, menu: &[&str]) -> Result<()> {
    macos.click_menu_item("Pro Tools", menu).await?;
    std::thread::sleep(std::time::Duration::from_millis(10)); // Wait 50ms
    Ok(())
}
async fn click_button(
    macos: &mut crate::macos::MacOSSession,
    window: &str,
    button: &str,
) -> Result<()> {
    macos.click_button("Pro Tools", window, button).await?;
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
    log::info!("=== solo_selected_tracks: START ===");

    log::info!("Fetching all tracks from Pro Tools...");
    let Some(tracks) = pt.get_all_tracks().await else {
        log::warn!("get_all_tracks returned None");
        return Ok(());
    };
    log::info!("Received {} tracks from Pro Tools", tracks.len());

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
                log::info!("  Track '{}': selected but not soloed -> will solo", name);
                solos.push(name.to_string());
            } else {
                log::info!("  Track '{}': soloed but not selected -> will unsolo", name);
                unsolos.push(name.to_string());
            }
        }
    }

    log::info!(
        "Soloing {} tracks, unsoloing {} tracks",
        solos.len(),
        unsolos.len()
    );

    if !solos.is_empty() {
        log::info!("Calling pt.solo_tracks for {} solos...", solos.len());
        pt.solo_tracks(solos, true).await?;
        log::info!("Solo tracks completed");
    }

    if !unsolos.is_empty() {
        log::info!("Calling pt.solo_tracks for {} unsolos...", unsolos.len());
        pt.solo_tracks(unsolos, false).await?;
        log::info!("Unsolo tracks completed");
    }

    log::info!("=== solo_selected_tracks: END ===");
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

pub async fn crossfade(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let preset = params.get_string("preset", "");
    let crossfade = params.get_bool("crossfade_automation", false);
    let fill = params.get_bool("fill_selection", false);
    let sr = pt.get_samplerate().await? / 1000;
    let adjust = params.get_int("adjust_selection_ms", 0) * sr;
    println!("adjustment frames: {}", adjust);
    let mut sel = PtSelectionSamples::new(pt).await?;
    let io = sel.get_io();
    sel.set_io(pt, io.0 - adjust, io.1 + adjust).await?;

    let mut macos = crate::macos::MacOSSession::new()?;

    if fill {
        call_menu(
            &mut macos,
            &["Edit", "Trim Clip", "Start to Fill Selection"],
        )
        .await?;
        call_menu(&mut macos, &["Edit", "Trim Clip", "End to Fill Selection"]).await?;
    }
    let result = pt
        .cmd::<_, serde_json::Value>(
            CommandId::CreateFadesBasedOnPreset,
            ptsl::CreateFadesBasedOnPresetRequestBody {
                fade_preset_name: preset,
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
    if crossfade {
        let c = sel.get_io();
        sel.set_io(pt, c.1, c.1).await?;
        call_menu(&mut macos, &["Edit", "Automation", "Write to All Enabled"]).await?;
        sel.set_io(pt, c.0, c.0).await?;
        call_menu(&mut macos, &["Edit", "Automation", "Write to All Enabled"]).await?;
        sel.set_io(pt, c.0 + 10, c.1 - 10).await?;

        let _: serde_json::Value = pt
            .cmd(
                CommandId::ClearSpecial,
                ptsl::ClearSpecialRequestBody {
                    automation_data_option: ptsl::AutomationDataOptions::AllAutomation.into(),
                },
            )
            .await?;

        // sel.set_io(pt, c.0 - 48000, c.1 + 48000).await?;
        // call_menu(&["Edit", "Automation", "Thin All"]).await?;
        // keystroke(&["cmd", "option", "control", "t"]).await?;
        sel.set_io(pt, c.0, c.1).await?;
    }
    Ok(())
}
pub async fn adjust_clip_to_match_selection(
    _pt: &mut ProtoolsSession,
    _params: &Params,
) -> Result<()> {
    let mut macos = crate::macos::MacOSSession::new()?;
    call_menu(&mut macos, &["Edit", "Trim Clip", "To Selection"]).await?;
    call_menu(&mut macos, &["Edit", "Trim Clip", "To Fill Selection"]).await?;
    call_menu(
        &mut macos,
        &["Edit", "Trim Clip", "Start to Fill Selection"],
    )
    .await?;
    call_menu(&mut macos, &["Edit", "Trim Clip", "End to Fill Selection"]).await?;
    Ok(())
}
pub async fn reset_clip(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    let mut macos = crate::macos::MacOSSession::new()?;
    call_menu(&mut macos, &["Edit", "Fades", "Delete"]).await?;
    call_menu(&mut macos, &["Edit", "Clear Special", "Clip Gain"]).await?;
    call_menu(&mut macos, &["Edit", "Clear Special", "Clip Effects"]).await?;
    let result: serde_json::Value = pt
        .cmd(
            CommandId::ClearSpecial,
            ptsl::ClearSpecialRequestBody {
                automation_data_option: ptsl::AutomationDataOptions::ClipGain.into(),
            },
        )
        .await?;
    println!("clip gain: {:?}", result);
    let result: serde_json::Value = pt
        .cmd(
            CommandId::ClearSpecial,
            ptsl::ClearSpecialRequestBody {
                automation_data_option: ptsl::AutomationDataOptions::ClipEffects.into(),
            },
        )
        .await?;
    println!("clip effects: {:?}", result);
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

    let mut macos = crate::macos::MacOSSession::new()?;
    call_menu(&mut macos, &["Edit", "Insert Silence"]).await?;
    // keystroke(&["cmd", "shift", "e"]).await?;
    // std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
    pt.set_edit_mode(&original_mode).await?;
    if flag {
        keystroke(&["cmd", "f1"]).await?;
    }
    Ok(())
}
pub async fn update_quick_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut number = params.get_int("number", 0);
    let default_text = format!("QM {}", number);
    let text = params.get_string("name", &default_text);
    let color = params.get_string("color", "magenta");
    number += 31000;
    let mut selection = PtSelectionSamples::new(pt).await?;
    selection.slide(pt, 48000).await?;
    let (st, et) = selection.get_io();
    pt.edit_marker(
        number as u32,
        &text,
        st,
        et,
        MarkerLocation::MainRuler,
        "",
        &color,
    )
    .await?;
    Ok(())
}

pub async fn go_to_quick_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut number = params.get_int("number", 0);
    number += 31000;
    let mut selection = PtSelectionSamples::new(pt).await?;
    let (st, et) = selection.get_io();
    let markers = pt.get_all_markers().await.unwrap_or(Vec::new());
    for marker in &markers {
        let marker_num = marker["number"].as_i64().unwrap_or(0);
        println!(
            "marker number vs requested number {}/{}",
            marker_num, number
        );
        if marker_num == number {
            println!("Success! marker: {:?}", marker);
            let start_time = marker["start_time"]
                .as_str()
                .unwrap_or("")
                .parse::<i64>()
                .unwrap_or(st);
            let end_time = marker["end_time"]
                .as_str()
                .unwrap_or("")
                .parse::<i64>()
                .unwrap_or(et);
            selection.set_io(pt, start_time, end_time).await?;
            return Ok(());
        }
    }
    Ok(())
}
/// Navigate to a marker with parameterized ruler name and direction
///
/// Parameters:
/// - `reverse`: boolean - true for previous marker, false for next marker (default: false)
/// - `ruler`: string - name of the marker ruler to use, empty string for all markers (default: "")
pub async fn go_to_next_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
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

async fn find_audiosuite_plugin(plugin_name: &str) -> Result<Vec<String>> {
    println!("Searching for {} in Audiosuite Menus", plugin_name);
    let menu_bar = crate::macos::menu::get_app_menus("Pro Tools")?;
    let audiosuite_menu = menu_bar
        .menus
        .iter()
        .find(|m| m.title == "AudioSuite")
        .ok_or_else(|| anyhow::anyhow!("AudioSuite menu not found"))?;

    for middleman in &audiosuite_menu.children {
        for category in &middleman.children {
            for middleman2 in &category.children {
                for plugin in &middleman2.children {
                    if plugin.title == plugin_name {
                        println!("found plugin location");
                        return Ok(vec![
                            "AudioSuite".to_string(),
                            category.title.clone(),
                            plugin_name.to_string(),
                        ]);
                    }
                }
            }
        }
    }
    for middleman in &audiosuite_menu.children {
        for category in &middleman.children {
            for middleman2 in &category.children {
                for plugin in &middleman2.children {
                    if crate::soft_match(&plugin.title, plugin_name) {
                        println!("found plugin location");
                        return Ok(vec![
                            "AudioSuite".to_string(),
                            category.title.clone(),
                            plugin_name.to_string(),
                        ]);
                    }
                }
            }
        }
    }
    println!("did not find plugin location");
    bail!("Plugin '{}' not found in AudioSuite menu", plugin_name)
}

async fn activate_plugin(macos: &mut crate::macos::MacOSSession, plugin_name: &str) -> Result<()> {
    let window = format!("AudioSuite: {}", plugin_name);

    // Check if already open
    if macos.window_exists("Pro Tools", &window).await? {
        println!("Plugin '{}' window already open", plugin_name);
        return Ok(());
    }

    let menu_path = find_audiosuite_plugin(plugin_name).await?;
    println!("menu path for {}:  {:?}", plugin_name, menu_path);
    // Convert Vec<String> to Vec<&str> for call_menu
    let menu_path_refs: Vec<&str> = menu_path.iter().map(|s| s.as_str()).collect();

    // Open it
    call_menu(macos, &menu_path_refs).await?;

    // Wait for window
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let timeout = Duration::from_millis(5000);
    while start.elapsed() < timeout {
        if macos
            .window_exists("Pro Tools", &window)
            .await
            .unwrap_or(false)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

pub async fn audiosuite(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let plugin = params.get_string("plugin", "");
    let button = params.get_string("button", "");
    let close = params.get_bool("close", false);
    call_audiosuite(&plugin, &button, close).await?;
    Ok(())
}
pub async fn call_audiosuite(plugin: &str, button: &str, close: bool) -> Result<()> {
    // Create MacOSSession once and reuse it (avoids multiple PID lookups)
    let mut macos = crate::macos::MacOSSession::new()?;

    // Activate plugin if specified
    if !plugin.is_empty() {
        activate_plugin(&mut macos, plugin).await?;
    }
    // Click button if specified
    if !button.is_empty() {
        let window = format!("AudioSuite: {}", plugin);
        click_button(&mut macos, &window, button).await?;
    }

    std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
    // Close window if requested
    if close {
        let window = format!("AudioSuite: {}", plugin);
        macos.close_window("Pro Tools", &window).await?;
    }

    Ok(())
}
pub async fn send_receive_rx(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let version = params.get_int("version", 11);
    let plugin = format!("RX {} Connect", version);
    let rx_app = format!("RX {}", version);
    let close = params.get_bool("close", false);

    let mut macos = crate::macos::MacOSSession::new()?;
    let app = macos.get_focused_app().await?;
    if app == "Pro Tools" {
        // Send to RX for analysis
        call_audiosuite(&plugin, "Analyze", false).await?;
    } else if crate::soft_match(&app, &rx_app) {
        // Send back to Pro Tools - Cmd+Enter returns to DAW
        keystroke(&["cmd", "enter"]).await?;

        // Wait for Pro Tools to actually get focus before proceeding
        let window = format!("AudioSuite: {}", plugin);
        use std::time::{Duration, Instant};
        let start = Instant::now();
        let timeout = Duration::from_millis(2000);
        while start.elapsed() < timeout {
            if let Ok(focused_app) = macos.get_focused_app().await {
                if focused_app == "Pro Tools" {
                    // Also verify window exists and is ready
                    if macos
                        .window_exists("Pro Tools", &window)
                        .await
                        .unwrap_or(false)
                    {
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Now render the changes back
        call_audiosuite(&plugin, "Render", close).await?;
    }

    Ok(())
}
lazy_static! {
    static ref PLUGIN_COUNTER: Arc<Mutex<HotkeyCounter>> =
        Arc::new(Mutex::new(HotkeyCounter::new()));
}
pub async fn multitap_plugin_selector(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut counter = PLUGIN_COUNTER.lock().unwrap();
    let plugins = params.get_string_vec("plugins");
    let timeout_ms = params.get_timeout_ms("timeout", 500);

    // Clone to move into the async closure
    let plugins_clone = plugins.clone();

    // Pass timeout, max, and async callback - cycle through all plugins (0-based indexing)
    counter.press(timeout_ms, plugins.len() as u32, |count| async move {
        if let Some(name) = plugins_clone.get(count as usize) {
            let menu_path = find_audiosuite_plugin(name).await;
            if let Ok(path) = menu_path {
                let mut macos = match crate::macos::MacOSSession::new() {
                    Ok(m) => m,
                    Err(e) => {
                        log::error!("Failed to create MacOSSession: {}", e);
                        return;
                    }
                };
                let refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
                let slice: &[&str] = &refs;
                if let Err(e) = call_menu(&mut macos, slice).await {
                    log::error!("Failed to open plugin {}: {:#}", name, e);
                }
            }
        }
    });

    Ok(())
}

pub async fn click_a_button(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let button = params.get_string("button", "");
    if button.is_empty() {
        return Ok(());
    };
    let mut macos = crate::macos::MacOSSession::new()?;
    click_button(&mut macos, "Edit", &button).await?;
    Ok(())
}
