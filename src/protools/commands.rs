use super::client::*;
use super::ptsl;
use anyhow::Result;
use crate::params::Params;
use ptsl::CommandId;

// ============================================================================
// Command Implementations
// ============================================================================

pub async fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    std::thread::sleep(std::time::Duration::from_millis(50)); // Wait 50ms
    Ok(())
}
pub async fn menu(menu: &[&str]) -> Result<()> {
    crate::macos::menu::run_menu_item("Pro Tools", menu)?;
    std::thread::sleep(std::time::Duration::from_millis(10)); // Wait 50ms
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

pub async fn crossfade_and_clear_automation(pt: &mut ProtoolsSession, preset: &str, _params: &Params) -> Result<()> {
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
    menu(&["Edit", "Automation", "Write to All Enabled"]).await?;
    // keystroke(&["cmd", "option", "slash"]).await?;
    sel.set_io(pt, c.0, c.0).await?;
    menu(&["Edit", "Automation", "Write to All Enabled"]).await?;
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
    menu(&["Edit", "Automation", "Thin All"]).await?;
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
    menu(&["Edit", "Insert Silence"]).await?;
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
    let ruler = params.get_str("ruler", "");
    pt.go_to_next_marker(&ruler, reverse).await?;
    Ok(())
}

// Legacy marker functions - kept for backward compatibility but deprecated
pub async fn go_to_next_marker(pt: &mut ProtoolsSession) -> Result<()> {
    pt.go_to_next_marker("", false).await?;
    Ok(())
}
pub async fn go_to_previous_marker(pt: &mut ProtoolsSession) -> Result<()> {
    pt.go_to_next_marker("", true).await?;
    Ok(())
}
pub async fn go_to_next_marker_1(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(0) {
        pt.go_to_next_marker(name, false).await?;
    }
    Ok(())
}

pub async fn go_to_previous_marker_1(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(0) {
        pt.go_to_next_marker(name, true).await?;
    }
    Ok(())
}
pub async fn go_to_next_marker_2(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(1) {
        pt.go_to_next_marker(name, false).await?;
    }
    Ok(())
}

pub async fn go_to_previous_marker_2(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(1) {
        pt.go_to_next_marker(name, true).await?;
    }
    Ok(())
}
pub async fn go_to_next_marker_3(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(2) {
        pt.go_to_next_marker(name, false).await?;
    }
    Ok(())
}

pub async fn go_to_previous_marker_3(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(2) {
        pt.go_to_next_marker(name, true).await?;
    }
    Ok(())
}
pub async fn go_to_next_marker_4(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(3) {
        pt.go_to_next_marker(name, false).await?;
    }
    Ok(())
}

pub async fn go_to_previous_marker_4(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(3) {
        pt.go_to_next_marker(name, true).await?;
    }
    Ok(())
}
pub async fn go_to_next_marker_5(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(4) {
        pt.go_to_next_marker(name, false).await?;
    }
    Ok(())
}

pub async fn go_to_previous_marker_5(pt: &mut ProtoolsSession) -> Result<()> {
    let rulers = pt.get_used_marker_ruler_names().await.unwrap_or(Vec::new());
    if let Some(name) = rulers.get(4) {
        pt.go_to_next_marker(name, true).await?;
    }
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
pub async fn spot_to_protools_from_soundminer(_pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Sending to Protools Session");
    crate::macos::menu::run_menu_item("Soundminer_Intel", &["Transfer", "Spot to DAW"])?;
    Ok(())
}
