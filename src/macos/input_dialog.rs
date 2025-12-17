//! Input dialog functionality via osascript (AppleScript)
//!
//! Provides a simple modal dialog with a text input field.
//! Uses AppleScript's display dialog command for simplicity and reliability.

use anyhow::Result;
use std::process::Command;

/// Show a modal input dialog and return the user's text input
///
/// This function displays a native macOS input dialog using osascript.
/// The dialog is modal and blocks until the user responds.
///
/// # Parameters
/// * `title` - The dialog title
/// * `prompt` - Optional prompt text (if None, uses title)
/// * `default_value` - Optional pre-filled text in the input field
///
/// # Returns
/// * `Ok(Some(String))` - User clicked OK with text (even if empty)
/// * `Ok(None)` - User clicked Cancel or closed dialog
/// * `Err(_)` - System error (osascript failure, etc.)
///
/// # Example
/// ```ignore
/// match show_input_dialog("Enter track name:", None, Some("Track 1"))? {
///     Some(text) => println!("User entered: {}", text),
///     None => println!("User cancelled"),
/// }
/// ```
pub fn show_input_dialog(
    title: &str,
    prompt: Option<&str>,
    default_value: Option<&str>,
) -> Result<Option<String>> {
    log::info!("Showing input dialog: '{}'", title);

    // Escape strings for AppleScript
    let escaped_title = title.replace("\"", "\\\"");
    let escaped_prompt = prompt.unwrap_or(title).replace("\"", "\\\"");
    let escaped_default = default_value.unwrap_or("").replace("\"", "\\\"");

    // Build AppleScript command
    let script = format!(
        "display dialog \"{}\" default answer \"{}\" with title \"{}\" buttons {{\"Cancel\", \"OK\"}} default button \"OK\"",
        escaped_prompt, escaped_default, escaped_title
    );

    // Execute osascript
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()?;

    // Check if user cancelled (exit code 1 with "User canceled" in stderr)
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") {
            log::info!("User cancelled dialog");
            return Ok(None);
        }
        anyhow::bail!("osascript failed: {}", stderr);
    }

    // Parse output: "button returned:OK, text returned:user input"
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract text between "text returned:" and end of line/string
    if let Some(text_start) = stdout.find("text returned:") {
        let text = stdout[text_start + 14..].trim().to_string();
        log::info!("User entered: '{}'", text);
        Ok(Some(text))
    } else {
        log::info!("User clicked OK with empty input");
        Ok(Some(String::new()))
    }
}
