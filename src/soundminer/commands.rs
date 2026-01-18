//! Soundminer command implementations
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
        OS::focus_app(d, "", false, true, 1000)?;
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
    OS::focus_app("Soundminer", "", true, true, 50).ok();
    if let Ok(sm) = OS::get_current_app() {
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
