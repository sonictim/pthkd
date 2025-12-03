//! Soundminer command implementations

use crate::params::Params;
use anyhow::{Result, Context};

// ============================================================================
// Command Implementations
// ============================================================================

pub fn spot_to_protools(_params: &Params) -> Result<()> {
    log::info!("Sending to Protools Session via Soundminer");
    crate::macos::menu::run_menu_item("Soundminer_Intel", &["Transfer", "Spot to DAW"])
        .context("Failed to spot to DAW")?;
    log::info!("✅ Spot to DAW command sent");
    Ok(())
}
