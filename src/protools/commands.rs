use super::client::*;
use super::ptsl;
use anyhow::Result;
use ptsl::CommandId;

pub async fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
    Ok(())
}

pub async fn solo_selected_tracks(pt: &mut ProtoolsSession) -> Result<()> {
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
        let is_selected = is_selected_str != "None";
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

/// Wrapper for crossfade with default preset (for use with pt_actions macro)
pub async fn crossfade(pt: &mut ProtoolsSession) -> Result<()> {
    crossfade_and_clear_automation(pt, "TF Default").await
}

pub async fn crossfade_and_clear_automation(pt: &mut ProtoolsSession, preset: &str) -> Result<()> {
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
    keystroke(&["cmd", "option", "slash"]).await?;
    sel.set_io(pt, c.0, c.0).await?;
    keystroke(&["cmd", "option", "slash"]).await?;
    sel.set_io(pt, c.0 + 100, c.1 - 100).await?;

    let _: serde_json::Value = pt
        .cmd(
            CommandId::ClearSpecial,
            ptsl::ClearSpecialRequestBody {
                automation_data_option: ptsl::AutomationDataOptions::AllAutomation.into(),
            },
        )
        .await?;

    sel.set_io(pt, c.0 - 4800, c.1 + 4800).await?;
    keystroke(&["cmd", "option", "control", "t"]).await?;
    sel.set_io(pt, c.0, c.1).await?;
    Ok(())
}

pub async fn conform_delete(pt: &mut ProtoolsSession) -> Result<()> {
    println!("Running Conform Delete");
    let mut flag = false;
    let original_mode = pt.get_edit_mode().await?;
    pt.set_edit_mode("EMO_Shuffle").await?;

    if pt.get_edit_mode().await? != "EMO_Shuffle" {
        keystroke(&["cmd", "f1"]).await?;
        std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
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
pub async fn conform_insert(pt: &mut ProtoolsSession) -> Result<()> {
    println!("Running Conform Insert");
    let mut flag = false;
    let original_mode = pt.get_edit_mode().await?;
    pt.set_edit_mode("EMO_Shuffle").await?;

    if pt.get_edit_mode().await? != "EMO_Shuffle" {
        keystroke(&["cmd", "f1"]).await?;
        std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
        pt.set_edit_mode("EMO_Shuffle").await?;
        flag = true;
    }

    keystroke(&["cmd", "shift", "e"]).await?;
    std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
    pt.set_edit_mode(&original_mode).await?;
    if flag {
        keystroke(&["cmd", "f1"]).await?;
    }
    Ok(())
}
pub async fn get_selection_samples(pt: &mut ProtoolsSession) -> Result<()> {
    let mut selection = PtSelectionSamples::new(pt).await?;
    selection.slide(pt, 48000).await?;
    Ok(())
}
