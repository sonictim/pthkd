use super::client::*;
use super::ptsl;
use crate::actions_async;
use crate::params::Params;
use anyhow::Result;

actions_async!("pt", tracks, {
    solo_selected,
    solo_clear,
    add_selected_to_solos,
    remove_selected_from_solos,
});

pub async fn solo_clear(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_soloed = attributes["is_soloed"].as_bool().unwrap_or(false);

        if is_soloed {
            solos.push(name.to_string());
        }
    }
    pt.solo_tracks(solos, false).await?;

    Ok(())
}

pub async fn solo_selected(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    log::info!("=== solo_selected_tracks: START ===");

    log::info!("Fetching all tracks from Pro Tools...");
    let Some(tracks) = pt.get_all_tracks().await else {
        log::warn!("get_all_tracks returned None");
        return Ok(());
    };
    log::info!("Received {} tracks from Pro Tools", tracks.len());

    let mut solos = Vec::new();
    let mut unsolos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_selected_str = attributes["is_selected"].as_str().unwrap_or("None");
        let is_selected = is_selected_str == "SetExplicitly";
        let is_soloed = attributes["is_soloed"].as_bool().unwrap_or(false);

        if is_soloed != is_selected {
            if is_selected {
                log::info!("  Track '{}': selected but not soloed -> will solo", name);
                solos.push(name.to_string());
            } else {
                log::info!("  Track '{}': soloed but not selected -> will unsolo", name);
                unsolos.push(name.to_string());
            }
        }
    }

    log::info!(
        "Soloing {} tracks, unsoloing {} tracks",
        solos.len(),
        unsolos.len()
    );

    if !solos.is_empty() {
        log::info!("Calling pt.solo_tracks for {} solos...", solos.len());
        pt.solo_tracks(solos, true).await?;
        log::info!("Solo tracks completed");
    }

    if !unsolos.is_empty() {
        log::info!("Calling pt.solo_tracks for {} unsolos...", unsolos.len());
        pt.solo_tracks(unsolos, false).await?;
        log::info!("Unsolo tracks completed");
    }

    log::info!("=== solo_selected_tracks: END ===");
    Ok(())
}

pub async fn add_selected_to_solos(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_selected_str = attributes["is_selected"].as_str().unwrap_or("None");
        let is_selected = is_selected_str != "None";

        if is_selected {
            solos.push(name.to_string());
        }
    }
    pt.solo_tracks(solos, true).await?;

    Ok(())
}
pub async fn remove_selected_from_solos(pt: &mut ProtoolsSession, _params: &Params) -> Result<()> {
    println!("Running Solo Selected Tracks");
    let Some(tracks) = pt.get_all_tracks().await else {
        return Ok(());
    };
    let mut solos = Vec::new();

    for track in tracks {
        let Some(name) = track["name"].as_str() else {
            continue;
        };
        let Some(attributes) = track["track_attributes"].as_object() else {
            continue;
        };
        let is_selected_str = attributes["is_selected"].as_str().unwrap_or("None");
        let is_selected = is_selected_str != "None";

        if is_selected {
            solos.push(name.to_string());
        }
    }
    pt.solo_tracks(solos, false).await?;

    Ok(())
}
