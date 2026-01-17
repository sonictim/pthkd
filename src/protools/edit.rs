//! ProTools edit actions (namespace: "pt")

use super::*;
use crate::actions_async;
use crate::prelude::*;
// Define all ProTools actions using the async macro
// Actions are automatically registered with the "pt" namespace
actions_async!("pt", edit, {
    crossfade,
    adjust_clip_to_match_selection,
    conform_delete,
    conform_insert,
    toggle_mode,
    toggle_tool,
    reset_clip,
    click_a_button,
    bg_paste_selection,
    bg_clear_selection,
});
use super::client::*;
use super::ptsl;
use super::timecode::*;
use crate::params::Params;
use ptsl::CommandId;

// ============================================================================
// Command Implementations
// ============================================================================

pub async fn crossfade(pt: &mut ProtoolsSession, params: &Params) -> R<()> {
    let preset = params.get_string("preset", "");
    let crossfade = params.get_bool("crossfade_automation", false);
    let fill = params.get_bool("fill_selection", false);
    let adjust = params.get_float("adjust_selection_frames", 0.0);
    let snap = params.get_bool("snap_to_grid", false);
    let mut sel = PtSelectionTimecode::new(pt).await?;
    if snap {
        let mut io = sel.get_io(pt).await?;
        io.0.snap_to_grid();
        io.1.snap_to_grid();
        sel.set_io(pt, &io.0, &io.1).await?;
    }
    if adjust > 0.0 {
        let mut io = sel.get_io(pt).await?;
        io.0.sub_hmsf(0, 0, 0, adjust);
        io.1.add_hmsf(0, 0, 0, adjust);
        sel.set_io(pt, &io.0, &io.1).await?;
    }
    if fill {
        OS::menu_click(
            "Pro Tools",
            &["Edit", "Trim Clip", "Start to Fill Selection"],
        )
        .ok();
        OS::menu_click("Pro Tools", &["Edit", "Trim Clip", "End to Fill Selection"]).ok();
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
        OS::menu_click("Pro Tools", &["Edit", "Automation", "Write to All Enabled"]).ok();

        let _: serde_json::Value = pt
            .cmd(
                CommandId::ClearSpecial,
                ptsl::ClearSpecialRequestBody {
                    automation_data_option: ptsl::AutomationDataOptions::AllAutomation.into(),
                },
            )
            .await?;
    }
    sel.set(pt).await?;
    Ok(())
}
pub async fn bg_paste_selection(pt: &mut ProtoolsSession, params: &Params) -> R<()> {
    let preset = params.get_string("fade_preset", "");
    let adjust = params.get_float("adjust_selection_frames", 0.0);
    let snap = params.get_bool("snap_to_grid", true);
    println!("adjustment frames: {}", adjust);
    let mut sel = PtSelectionTimecode::new(pt).await?;
    if snap {
        let mut io = sel.get_io(pt).await?;
        io.0.snap_to_grid();
        io.1.snap_to_grid();
        sel.set_io(pt, &io.0, &io.1).await?;
    }
    if adjust > 0.0 {
        let mut io = sel.get_io(pt).await?;
        io.0.sub_hmsf(0, 0, 0, adjust);
        io.1.add_hmsf(0, 0, 0, adjust);
        sel.set_io(pt, &io.0, &io.1).await?;
    }
    pt.paste_to_fill_selection().await?;
    if !preset.is_empty() {
        pt.cmd::<_, serde_json::Value>(
            CommandId::CreateFadesBasedOnPreset,
            ptsl::CreateFadesBasedOnPresetRequestBody {
                fade_preset_name: preset,
                auto_adjust_bounds: true,
            },
        )
        .await
        .ok();
    }
    // sel.set_io(pt, &io.0, &io.1).await?;
    // adjust_clip_to_match_selection(pt, params).await?;
    Ok(())
}
pub async fn bg_clear_selection(pt: &mut ProtoolsSession, params: &Params) -> R<()> {
    let adjust = params.get_float("adjust_selection_frames", 0.0);
    let snap = params.get_bool("snap_to_grid", true);
    println!("adjustment frames: {}", adjust);
    let mut sel = PtSelectionTimecode::new(pt).await?;
    if snap {
        let mut io = sel.get_io(pt).await?;
        io.0.snap_to_grid();
        io.1.snap_to_grid();
        sel.set_io(pt, &io.0, &io.1).await?;
    }
    if adjust > 0.0 {
        let mut io = sel.get_io(pt).await?;
        io.0.add_hmsf(0, 0, 0, adjust);
        io.1.sub_hmsf(0, 0, 0, adjust);
        sel.set_io(pt, &io.0, &io.1).await?;
    }
    pt.clear().await?;
    Ok(())
}
pub async fn adjust_clip_to_match_selection(_pt: &mut ProtoolsSession, _params: &Params) -> R<()> {
    OS::menu_click("Pro Tools", &["Edit", "Trim Clip", "To Selection"]).ok();
    OS::menu_click("Pro Tools", &["Edit", "Trim Clip", "To Fill Selection"]).ok();
    OS::menu_click(
        "Pro Tools",
        &["Edit", "Trim Clip", "Start to Fill Selection"],
    )
    .ok();
    OS::menu_click("Pro Tools", &["Edit", "Trim Clip", "End to Fill Selection"]).ok();
    Ok(())
}
pub async fn reset_clip(pt: &mut ProtoolsSession, _params: &Params) -> R<()> {
    OS::menu_click("Pro Tools", &["Edit", "Fades", "Delete"]).ok();
    OS::menu_click("Pro Tools", &["Edit", "Clear Special", "Clip Gain"]).ok();
    OS::menu_click("Pro Tools", &["Edit", "Clear Special", "Clip Effects"]).ok();
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
pub async fn conform_delete(pt: &mut ProtoolsSession, _params: &Params) -> R<()> {
    println!("Running Conform Delete");
    let mut flag = false;
    let original_mode = pt.get_edit_mode().await?;
    pt.set_edit_mode("EMO_Shuffle").await?;

    if pt.get_edit_mode().await? != "EMO_Shuffle" {
        OS::keystroke(&["cmd", "f1"])?;
        // std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
        pt.set_edit_mode("EMO_Shuffle").await?;
        flag = true;
    }
    let _: serde_json::Value = pt.cmd(CommandId::Clear, serde_json::json!({})).await?;
    // std::thread::sleep(std::time::Duration::from_millis(25)); // Wait 50ms
    pt.set_edit_mode(&original_mode).await?;
    if flag {
        OS::keystroke(&["cmd", "f1"])?;
    }
    Ok(())
}
pub async fn conform_insert(pt: &mut ProtoolsSession, _params: &Params) -> R<()> {
    println!("Running Conform Insert");
    let mut flag = false;
    let original_mode = pt.get_edit_mode().await?;
    pt.set_edit_mode("EMO_Shuffle").await?;

    if pt.get_edit_mode().await? != "EMO_Shuffle" {
        OS::keystroke(&["cmd", "f1"])?;
        // std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
        pt.set_edit_mode("EMO_Shuffle").await?;
        flag = true;
    }
    OS::menu_click("Pro Tools", &["Edit", "Insert Silence"])?;
    // OS::keystroke(&["cmd", "shift", "e"]).await?;
    std::thread::sleep(std::time::Duration::from_millis(35)); // Wait 50ms
    pt.set_edit_mode(&original_mode).await?;
    if flag {
        OS::keystroke(&["cmd", "f1"])?;
    }
    Ok(())
}
pub async fn toggle_mode(pt: &mut ProtoolsSession, _params: &Params) -> R<()> {
    let mode = pt.get_edit_mode().await?;
    if mode != "EMO_GridAbsolute" {
        pt.set_edit_mode("EMO_GridAbsolute").await?;
    } else {
        pt.set_edit_mode("EMO_Slip").await?;
    }
    Ok(())
}
pub async fn toggle_tool(pt: &mut ProtoolsSession, _params: &Params) -> R<()> {
    let tool = pt.get_edit_tool().await?;
    if tool != "ET_Selector" {
        pt.set_edit_tool("ET_Selector").await?;
    } else {
        pt.set_edit_tool("ET_GrabberTime").await?;
    }
    Ok(())
}
pub async fn click_a_button(_pt: &mut ProtoolsSession, params: &Params) -> R<()> {
    let button = params.get_string("button", "");
    if button.is_empty() {
        return Ok(());
    };
    OS::click_button("Pro Tools", "Edit", &button)?;
    Ok(())
}
