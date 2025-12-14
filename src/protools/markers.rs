use super::client::*;
use super::{keystroke, ptsl};
use crate::params::Params;
use anyhow::Result;

use crate::actions_async;

actions_async!("pt", markers, {
    go_to_next_marker,
    go_to_quick_marker,
    update_quick_marker,
});

pub async fn update_quick_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut number = params.get_int("number", 0);
    let default_text = format!("QM {}", number);
    let text = params.get_string("name", &default_text);
    let color = params.get_string("color", "magenta");
    number += 31000;
    let mut selection = PtSelectionSamples::new(pt).await?;
    selection.slide(pt, 48000).await?;
    let (st, et) = selection.get_io();
    pt.edit_marker(
        number as u32,
        &text,
        st,
        et,
        MarkerLocation::MainRuler,
        "",
        &color,
    )
    .await?;
    Ok(())
}

pub async fn go_to_quick_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let mut number = params.get_int("number", 0);
    number += 31000;
    let mut selection = PtSelectionSamples::new(pt).await?;
    let (st, et) = selection.get_io();
    let markers = pt.get_all_markers().await.unwrap_or(Vec::new());
    for marker in &markers {
        let marker_num = marker["number"].as_i64().unwrap_or(0);
        println!(
            "marker number vs requested number {}/{}",
            marker_num, number
        );
        if marker_num == number {
            println!("Success! marker: {:?}", marker);
            let start_time = marker["start_time"]
                .as_str()
                .unwrap_or("")
                .parse::<i64>()
                .unwrap_or(st);
            let end_time = marker["end_time"]
                .as_str()
                .unwrap_or("")
                .parse::<i64>()
                .unwrap_or(et);
            selection.set_io(pt, start_time, end_time).await?;
            return Ok(());
        }
    }
    Ok(())
}
/// Navigate to a marker with parameterized ruler name and direction
///
/// Parameters:
/// - `reverse`: boolean - true for previous marker, false for next marker (default: false)
/// - `ruler`: string - name of the marker ruler to use, empty string for all markers (default: "")
pub async fn go_to_next_marker(pt: &mut ProtoolsSession, params: &Params) -> Result<()> {
    let reverse = params.get_bool("reverse", false);
    let ruler = params.get_string("ruler", "");
    pt.go_to_next_marker(&ruler, reverse).await?;
    keystroke(&["left"]).await?;
    Ok(())
}
