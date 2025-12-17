use super::client::*;
use crate::macos::menu::*;
use crate::params::Params;
use anyhow::Result;

use crate::actions_async;

actions_async!("pt", session, {
     export_selection,
});

pub async fn export_selection(_pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    if !menu_item_enabled("Pro Tools", &["Options", "Link Track and Edit Selection"]) {
        menu_item_run("Pro Tools", &["Options", "Link Track and Edit Selection"])?;
    }
    menu_item_run("Pro Tools", &["File", "Save Session Copy In..."])?;
    let audio_files = params.get_bool("copy_audio_files", false);

    crate::macos::ui_elements::click_checkbox("Pro Tools", "Save Copy In..", "Main Playlist Only")?;
    crate::macos::ui_elements::click_checkbox(
        "Pro Tools",
        "Save Copy In..",
        "Selected Tracks Only",
    )?;
    crate::macos::ui_elements::click_checkbox(
        "Pro Tools",
        "Save Copy In..",
        "Selected Timeline Range Only",
    )?;
    if audio_files {
        crate::macos::ui_elements::click_checkbox("Pro Tools", "Save Copy In..", "Audio Files")?;
    }
    Ok(())
}
