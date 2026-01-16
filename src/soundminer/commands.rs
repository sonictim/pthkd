//! Soundminer command implementations

use crate::params::Params;
use crate::prelude::*;

// ============================================================================
// Command Implementations
// ============================================================================

pub fn send_to_daw(params: &Params) -> R<()> {
    let daw = params.get_ostring("daw");
    let command = params.get_str("command", "Bring into DAW");
    let refo = params.get_obool("reference_original");
    let orig = params.get_obool("original_sample_rate");
    let sprn = params.get_obool("spot_as_region");
    let launch = params.get_bool("launch", false);
    // let sm = crate::soft_match_vec("Soundminer", &OS::get_running_apps()?)
    //     .unwrap_or("Soundminer".to_string());
    // println!(
    //     "Running Apps: {:?}",
    //     OS::get_running_apps()
    // );
    if launch && let Some(ref d) = daw {
        crate::macos::app_info::focus_app(d, "", false, true, 1000)?;
    }
    let sm = focus_sm();

    send_sm_event("refo", refo)?;
    send_sm_event("orig", orig)?;
    send_sm_event("sprn", sprn)?;
    if let Some(ref d) = daw {
        OS::menu_click(&sm, &["DAW", d])?;
    }
    OS::menu_click(&sm, &["Transfer", command])?;
    log::info!("✅ Spot to DAW command sent");
    Ok(())
}

// pub fn send_to_daw_old(params: &Params) -> R<()> {
//     let mut daw = params.get_ostring("daw");
//     if daw == Some("current".to_string()) {
//         let app = crate::macos::app_info::get_current_app()
//             .ok()
//             .unwrap_or_default();
//
//         daw = Some(app);
//     }
//     let mut command = params.get_str("command", "Bring into DAW");
//     let mut refo = params.get_obool("reference_original");
//     let mut orig = params.get_obool("original_sample_rate");
//     let mut sprn = params.get_obool("spot_as_region");
//
//     // Check if Soundminer is running - if not, just launch it and return
//     let apps = crate::macos::app_info::get_all_running_applications()?;
//     let soundminer_running = apps.iter().any(|app| crate::soft_match(app, "Soundminer"));
//
//     if !soundminer_running {
//         log::info!("Soundminer not running, launching...");
//         crate::macos::app_info::launch_application("Soundminer")?;
//         log::info!("✅ Soundminer launched (no files to send)");
//         return Ok(());
//     }
//
//     // Soundminer is running, focus it and execute the command
//     let sm = focus_sm();
//
//     match daw.as_deref() {
//         Some("Pro Tools") => {
//             log::info!("Spotting to Protools Timeline via Soundminer");
//             if refo.is_none() {
//                 refo = Some(false);
//             }
//             if orig.is_none() {
//                 orig = Some(false);
//             }
//             if sprn.is_none() {
//                 sprn = Some(true)
//             };
//             OS::menu_click(&sm, &["DAW", "Pro Tools"]).ok();
//         }
//         Some("Reaper") => {
//             log::info!("Spotting to Reaper Timeline via Soundminer");
//             if refo.is_none() {
//                 refo = Some(false);
//             }
//             if orig.is_none() {
//                 orig = Some(true);
//             }
//             if sprn.is_none() {
//                 sprn = Some(true)
//             };
//             OS::menu_click(&sm, &["DAW", "Soundminer Reaper Extension"]).ok();
//         }
//         Some("iZotope RX 11") => {
//             command = "Send Files to DAW";
//             let apps = crate::macos::app_info::get_all_running_applications()?;
//             if !apps.contains(&"iZotope RX 11".to_string()) {
//                 crate::macos::app_info::launch_application("iZotope RX 11 Audio Editor").ok();
//                 OS::wait_for_window(
//                     "iZotope RX 11 Audio Editor",
//                     "",
//                     OS::WindowCondition::Focused,
//                     500,
//                 )
//                 .ok();
//                 crate::macos::app_info::focus_app("Soundminer", "", true, false, 50).ok();
//             }
//             OS::menu_click(&sm, &["DAW", "iZotope RX 11"]).ok();
//         }
//         _ => {}
//     }
//
//     send_sm_event("refo", refo)?;
//     send_sm_event("orig", orig)?;
//     send_sm_event("sprn", sprn)?;
//     OS::menu_click(&sm, &["Transfer", command])?;
//     log::info!("✅ Spot to DAW command sent");
//     Ok(())
// }

pub fn send_sm_event(id: &str, param: Option<bool>) -> R<()> {
    if let Some(param) = param {
        let p = if param { 1 } else { 0 };
        send_apple_event("Soundminer v6", "SNDM", id, p)
    } else {
        Ok(())
    }
}

pub fn send_apple_event(app: &str, event_class: &str, event_id: &str, param: i32) -> R<()> {
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
    let r = OS::click_button("Soundminer_Intel", "", "setTransfer");
    println!("select folder: {:?}", r);
    Ok(())
}

fn focus_sm() -> String {
    crate::macos::app_info::focus_app("Soundminer", "", true, true, 50).ok();
    if let Ok(sm) = crate::macos::app_info::get_current_app() {
        sm
    } else {
        "Soundminer_Intel".to_string()
    }
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
