use super::client::*;
use super::ptsl;
use anyhow::Result;
use ptsl::CommandId;

pub async fn keystroke(keys: &[&str]) -> Result<()> {
    crate::macos::keystroke::send_keystroke(keys)?;
    std::thread::sleep(std::time::Duration::from_millis(50)); // Wait 50ms
    Ok(())
}
pub async fn menu(menu: &[&str]) -> Result<()> {
    crate::macos::menu::run_menu_item("Soundminer_Intel", menu)?;
    // std::thread::sleep(std::time::Duration::from_millis(10)); // Wait 50ms
    Ok(())
}
pub async fn spot_to_protools() -> Result<()> {
    println!("Sending to Protools Session");
    menu(&["DAW", "Pro Tools"])?;
    menu(&["Transfer", "Spot To DAW"])?;
    Ok(())
}
