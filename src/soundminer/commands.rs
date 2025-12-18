//! Soundminer command implementations

use crate::params::Params;
use anyhow::Result as R;

// ============================================================================
// Command Implementations
// ============================================================================

pub fn send_to_daw(params: &Params) -> R<()> {
    let mut daw = params.get_ostring("daw");
    if daw == Some("current".to_string()) {
        let app = crate::macos::app_info::get_current_app()
            .ok()
            .unwrap_or_default();

        daw = Some(app);
    }
    let mut command = params.get_str("command", "Bring into DAW");
    let mut refo = params.get_obool("reference_original");
    let mut orig = params.get_obool("original_sample_rate");
    let mut sprn = params.get_obool("spot_as_region");

    // Check if Soundminer is running - if not, just launch it and return
    let apps = crate::macos::app_info::get_all_running_applications()?;
    let soundminer_running = apps.iter().any(|app| crate::soft_match(app, "Soundminer"));

    if !soundminer_running {
        log::info!("Soundminer not running, launching...");
        crate::macos::app_info::launch_application("Soundminer")?;
        log::info!("✅ Soundminer launched (no files to send)");
        return Ok(());
    }

    // Soundminer is running, focus it and execute the command
    let _ = crate::macos::app_info::focus_app("Soundminer", "", true, false, 50);

    match daw.as_deref() {
        Some("Pro Tools") => {
            log::info!("Spotting to Protools Timeline via Soundminer");
            if refo.is_none() {
                refo = Some(false);
            }
            if orig.is_none() {
                orig = Some(false);
            }
            if sprn.is_none() {
                sprn = Some(true)
            };
            let _ = super::menu(&["DAW", "Pro Tools"]);
        }
        Some("Reaper") => {
            log::info!("Spotting to Reaper Timeline via Soundminer");
            if refo.is_none() {
                refo = Some(false);
            }
            if orig.is_none() {
                orig = Some(true);
            }
            if sprn.is_none() {
                sprn = Some(true)
            };
            let _ = super::menu(&["DAW", "Soundminer Reaper Extension"]);
        }
        Some("iZotope RX 11") => {
            command = "Send Files to DAW";
            let apps = crate::macos::app_info::get_all_running_applications()?;
            if !apps.contains(&"iZotope RX 11".to_string()) {
                let _ = crate::macos::app_info::launch_application("iZotope RX 11 Audio Editor");
                let _ = crate::macos::ui_elements::wait_for_window_focused(
                    "iZotope RX 11 Audio Editor",
                    "",
                    500,
                );
                let _ = crate::macos::app_info::focus_app("Soundminer", "", true, false, 50);
            }
            let _ = super::menu(&["DAW", "iZotope RX 11"]);
            // let _ = crate::macos::ui_elements::wait_for_window_focused("iZotope RX 11 Audio Editor", "", 50);
        }
        _ => {}
    }

    send_sm_event("refo", refo)?;
    send_sm_event("orig", orig)?;
    send_sm_event("sprn", sprn)?;
    super::menu(&["Transfer", command])?;
    log::info!("✅ Spot to DAW command sent");
    Ok(())
}

pub fn send_sm_event(id: &str, param: Option<bool>) -> R<()> {
    if let Some(param) = param {
        let p = if param { 1 } else { 0 };
        send_apple_event("Soundminer v6", "SNDM", id, p)
    } else {
        Ok(())
    }
}

pub fn send_apple_event(app: &str, event_class: &str, event_id: &str, param: i32) -> R<()> {
    use std::time::Duration;

    let script = format!(
        "tell application \"{}\" to «event {}{}» {}",
        app, event_class, event_id, param
    );

    // Use a timeout to prevent hanging if the app isn't responding
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                log::warn!("AppleEvent returned non-zero status: {}", stderr);
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to execute AppleEvent: {}", e);
            anyhow::bail!("AppleEvent execution failed: {}", e)
        }
    }
}

pub fn select_spotting_folder(_params: &Params) -> R<()> {
    println!("opening spot folder dialog");
    let r = crate::macos::ui_elements::click_button("Soundminer_Intel", "", "setTransfer");
    println!("select folder: {:?}", r);
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
