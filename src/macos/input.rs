//! Input dialog implementation for MacOSSession
//!
//! Provides async modal input dialogs using AppleScript (simple and thread-safe)

use super::session::MacOSSession;
use anyhow::Result;
use std::process::Command;

impl MacOSSession {
    /// Show input dialog and await result asynchronously
    ///
    /// Uses osascript (AppleScript) which handles threading correctly
    ///
    /// # Returns
    /// * `Ok(Some(String))` - User clicked OK with text (even if empty)
    /// * `Ok(None)` - User clicked Cancel
    /// * `Err(_)` - System error
    pub async fn show_input_dialog(
        &mut self,
        title: &str,
        prompt: Option<&str>,
        default_value: Option<&str>,
    ) -> Result<Option<String>> {
        log::info!("Showing input dialog via osascript: '{}'", title);

        let prompt_text = prompt.unwrap_or("");
        let default_text = default_value.unwrap_or("");

        // Build AppleScript command
        let script = format!(
            r#"display dialog "{}" with title "{}" default answer "{}""#,
            prompt_text.replace('"', "\\\""),
            title.replace('"', "\\\""),
            default_text.replace('"', "\\\"")
        );

        // Run osascript asynchronously (spawn_blocking to avoid blocking tokio runtime)
        let result = tokio::task::spawn_blocking(move || {
            Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

        if result.status.success() {
            // Parse output: "button returned:OK, text returned:user input"
            let output = String::from_utf8_lossy(&result.stdout);

            // Extract text after "text returned:"
            if let Some(text_start) = output.find("text returned:") {
                let text = output[text_start + 14..].trim().to_string();
                log::info!("User entered: '{}'", text);
                Ok(Some(text))
            } else {
                Ok(Some(String::new()))
            }
        } else {
            // User clicked Cancel or error occurred
            let stderr = String::from_utf8_lossy(&result.stderr);
            if stderr.contains("User canceled") {
                log::info!("User cancelled dialog");
                Ok(None)
            } else {
                anyhow::bail!("osascript error: {}", stderr)
            }
        }
    }
}
