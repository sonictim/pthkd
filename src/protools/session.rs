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
    let mut tag = params.get_str("name_id", "");
    let path = pt.get_session_path().await?;
    println!("source path: {}", path.display());
    let source_name = path.file_stem().unwrap().to_str().unwrap();
    let location = path.parent().unwrap().to_str().unwrap();
    let mut v: Vec<&str> = source_name.split('_').collect();
    let id = v.pop().unwrap_or_default();
    let number = v.pop().unwrap_or_default().parse::<u8>().ok();
    if let Some(mut num) = number {
        if tag.is_empty() {
            tag = id;
        }
        num += 1;
        let mut n = String::new();
        for s in v {
            n.push_str(s);
            n.push('_');
        }
        n.push_str(&format!("{:02}", num));
        n.push('_');
        n.push_str(tag);
        pt.save_session_as(&n, location).await?;
    }

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
