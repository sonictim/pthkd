//! Soundminer command implementations

use crate::params::Params;
use anyhow::{Context, Result};

// ============================================================================
// Command Implementations
// ============================================================================

pub fn spot_to_protools(_params: &Params) -> Result<()> {
    log::info!("Sending to Protools Session via Soundminer");
    send_sm_event("refo", 0)?;
    send_sm_event("orig", 0)?;
    send_sm_event("sprn", 1)?;
    let _ = crate::macos::app_info::focus_application("Soundminer");
    let _ = crate::macos::ui_elements::wait_for_window_focused("Soundminer", "Soundminer", 50);

    let _ = crate::macos::menu::menu_item_run("Soundminer", &["DAW", "Pro Tools"]);
    crate::macos::menu::menu_item_run("Soundminer", &["Transfer", "Spot To DAW"])?;
    log::info!("✅ Spot to DAW command sent");
    Ok(())
}

pub fn send_sm_event(id: &str, param: i32) -> Result<()> {
    send_apple_event("Soundminer v6", "SNDM", id, param)
}

pub fn send_apple_event(app: &str, event_class: &str, event_id: &str, param: i32) -> Result<()> {
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
