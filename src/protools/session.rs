use super::client::*;
use crate::macos::menu::*;
use crate::macos::ui_elements::*;
use crate::params::Params;
use anyhow::Result;

use crate::actions_async;

actions_async!("pt", session, {
     export_selection,
    popups,
    version_up,
    save_as,
});
pub async fn save_as(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let name = params.get_str("name", "");
    let location = params.get_str("location", "");

    if !name.is_empty() && !location.is_empty() {
        pt.save_session_as(name, location).await?;
    }
    Ok(())
}
pub async fn version_up(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let tag_param = params.get_str("name_id", "");

    let path = pt.get_session_path().await?;
    println!("source path: {}", path.display());

    let parent = path
        .parent()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Session path has no parent"))?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid session filename"))?;

    let mut parts: Vec<&str> = stem.split('_').collect();

    let id = parts.pop().unwrap_or_default();
    let mut num: u8 = parts
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Missing version number"))?
        .parse()?;

    let tag = if tag_param.is_empty() { id } else { tag_param };

    num += 1;

    let new_name = format!("{}_{:02}_{}", parts.join("_"), num, tag);

    pt.save_session_as(&new_name, parent).await?;

    Ok(())
}
pub async fn export_selection(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    if !menu_item_enabled("Pro Tools", &["Options", "Link Track and Edit Selection"]) {
        menu_item_run("Pro Tools", &["Options", "Link Track and Edit Selection"])?;
    }
    menu_item_run("Pro Tools", &["File", "Save Session Copy In..."])?;
    wait_for_window_exists("Pro Tools", "Save Copy In...", 200)?;
    let audio_files = params.get_bool("copy_audio_files", false);
    if audio_files {
        click_checkbox("Pro Tools", "Save Copy In..", "Audio Files")?;
    }
    click_checkbox("Pro Tools", "Save Copy In...", "Main Playlist Only")?;
    click_checkbox("Pro Tools", "Save Copy In...", "Selected Tracks Only")?;
    click_checkbox(
        "Pro Tools",
        "Save Copy In...",
        "Selected Timeline Range Only",
    )?;
    let close = params.get_bool("close", true);
    if close {
        click_button("Pro Tools", "Save Copy In...", "Ok")?;
    }
    Ok(())
}

pub async fn popups(_pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    match crate::macos::ui_elements::get_popup_menu_items(
        "Pro Tools",
        "", // Empty string = focused window
        "Grid Value",
    ) {
        Ok(items) => {
            println!("✅ Found {} popup items:", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("  {}. {}", i + 1, item);
            }
        }
        Err(e) => {
            println!("❌ Error getting popup items: {}", e);
            return Err(e);
        }
    }
    Ok(())
}
