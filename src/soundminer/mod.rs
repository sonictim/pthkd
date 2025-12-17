//! Soundminer integration module

pub mod commands;
pub mod actions;

use anyhow::Result;

fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    std::thread::sleep(std::time::Duration::from_millis(50)); // Wait 50ms
    Ok(())
}

fn menu(menu: &[&str]) -> Result<()> {
    crate::macos::menu::menu_item_run("Soundminer_Intel", menu)?;
    // std::thread::sleep(std::time::Duration::from_millis(10)); // Wait 50ms
    Ok(())
}

pub fn spot_to_protools() {
    println!("Sending to Protools Session");
    let _ = crate::macos::app_info::focus_application("Soundminer");
    let _ = menu(&["DAW", "Pro Tools"]);
    let _ = menu(&["Transfer", "Spot To DAW"]);
}
