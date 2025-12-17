//! Soundminer command implementations

use crate::params::Params;
use anyhow::Result as R;

// ============================================================================
// Command Implementations
// ============================================================================

pub fn spot_to_daw(params: &Params) -> R<()> {
    let mut daw = params.get_ostr("daw");
    if daw.is_none() {
        daw = crate::macos::app_info::get_current_app().ok().as_deref();
    }
    let _ = crate::macos::app_info::focus_application("Soundminer");
    let _ = crate::macos::ui_elements::wait_for_window_focused("Soundminer", "Soundminer", 50);
    match app.as_str() {
        "Pro Tools" => {
            log::info!("Spotting to Protools Timeline via Soundminer");
            send_sm_event("refo", 0)?;
            send_sm_event("orig", 0)?;
            send_sm_event("sprn", 1)?;
            let _ = crate::macos::menu::menu_item_run("Soundminer", &["DAW", "Pro Tools"]);
        }
        "Reaper" => {
            log::info!("Spotting to Reaper Timeline via Soundminer");
            send_sm_event("refo", 0)?;
            send_sm_event("orig", 1)?;
            send_sm_event("sprn", 1)?;
            let _ = crate::macos::menu::menu_item_run(
                "Soundminer",
                &["DAW", "Soundminer Reaper Extension"],
            );
        }
        _ => {}
    }

    crate::macos::menu::menu_item_run("Soundminer", &["Transfer", "Spot To DAW"])?;
    log::info!("✅ Spot to DAW command sent");
    Ok(())
}

pub fn send_to_daw(params: &Params) -> R<()> {
    let _ = crate::macos::app_info::focus_application("Soundminer");
    let _ = crate::macos::ui_elements::wait_for_window_focused("Soundminer", "Soundminer", 50);
    log::info!("Sending to DAW via Soundminer");
    let so = params.get_bool("selection_only", false);
    let so = if so { 0 } else { 1 };
    send_sm_event("refo", 0)?;
    send_sm_event("orig", 0)?;
    send_sm_event("sprn", so)?;

    let _ = crate::macos::menu::menu_item_run("Soundminer", &["DAW", "Pro Tools"]);
    crate::macos::menu::menu_item_run("Soundminer", &["Transfer", "Bring into DAW"])?;
    log::info!("✅ Spot to DAW command sent");
    Ok(())
}

pub fn send_sm_event(id: &str, param: i32) -> R<()> {
    send_apple_event("Soundminer v6", "SNDM", id, param)
}

pub fn send_apple_event(app: &str, event_class: &str, event_id: &str, param: i32) -> R<()> {
    let script = format!(
        "tell application \"{}\" to «event {}{}» {}",
        app, event_class, event_id, param
    );

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()?;

    Ok(())
}

// SM Apple codes:
//
// refo - refernce original
// orig - original samplerate / bit depth
// sprn - spot as region
// iqtf - intelligent transfers
// slct - select spotting folder
//
// wfex - execute a workflow
// wfls - list all workflows
// wfrn - workflows
//
// gmet - get selected
//
// rtog - radium toggle
// mbed - embed
//
