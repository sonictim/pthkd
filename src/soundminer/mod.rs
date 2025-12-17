//! Soundminer integration module

pub mod actions;
pub mod commands;

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
