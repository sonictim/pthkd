use super::client::*;
use crate::macos::menu::*;
use crate::macos::ui_elements::*;
use crate::params::Params;
use anyhow::Result;

use crate::actions_async;

actions_async!("pt", session, {
     export_selection,
    popups,
});

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
