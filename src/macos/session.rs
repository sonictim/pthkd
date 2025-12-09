//! Central session for macOS operations
//!
//! Similar to ProtoolsSession, this provides a unified interface for all macOS
//! automation tasks. All macOS commands take &mut MacOSSession as first parameter.

use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;

/// Central session for macOS operations
///
/// Provides a unified interface for macOS automation similar to ProtoolsSession.
/// Caches PIDs and other state to avoid repeated lookups.
///
/// # Example
/// ```ignore
/// let mut macos = MacOSSession::new()?;
/// macos.focus_app("Pro Tools").await?;
/// macos.click_button("Pro Tools", "Fade", "OK").await?;
/// ```
pub struct MacOSSession {
    /// Cache of app name -> (PID, timestamp) to avoid repeated lookups
    cached_pids: HashMap<String, (i32, Instant)>,

    /// Whether accessibility API is confirmed working
    accessibility_enabled: bool,
}

impl MacOSSession {
    /// Create new macOS session
    pub fn new() -> Result<Self> {
        Ok(Self {
            cached_pids: HashMap::new(),
            accessibility_enabled: false,
        })
    }

    /// Get PID for app, using cache when possible
    ///
    /// Cache is valid for 5 seconds to balance freshness with performance
    pub fn get_pid(&mut self, app_name: &str) -> Result<i32> {
        // Check cache (valid for 5 seconds)
        if let Some((pid, timestamp)) = self.cached_pids.get(app_name) {
            if timestamp.elapsed().as_secs() < 5 {
                return Ok(*pid);
            }
        }

        // Look up fresh PID
        let pid = super::ffi::get_pid_by_name(app_name)?;
        self.cached_pids.insert(app_name.to_string(), (pid, Instant::now()));
        Ok(pid)
    }

    /// Clear PID cache (useful after launching/quitting apps)
    pub fn clear_cache(&mut self) {
        self.cached_pids.clear();
    }
}

impl Drop for MacOSSession {
    fn drop(&mut self) {
        // Future: Release any cached AX elements here
        log::debug!("MacOSSession dropped, cache had {} entries", self.cached_pids.len());
    }
}
