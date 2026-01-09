//! Soundminer integration module

pub mod actions;
pub mod commands;

use anyhow::Result;

fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    Ok(())
}

fn menu(menu: &[&str]) -> Result<()> {
    crate::swift_bridge::menu_click("Soundminer_Intel", menu)?;
    Ok(())
}
