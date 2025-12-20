//! macOS Permission Handling Module
//!
//! Provides blocking permission dialog and checks for:
//! - Accessibility (required for event tap creation)
//! - Input Monitoring (required for keystroke sending)

use super::ffi::{AXIsProcessTrusted, CGRequestPostEventAccess};
use super::session::MacOSSession;
use anyhow::{Context, Result};
use objc2::msg_send;
use objc2::runtime::AnyObject;
use std::process::Command;
use std::thread;
use std::time::Duration;

// ============================================================================
// Permission State Types
// ============================================================================

/// Result from permission dialog
#[derive(Debug, Clone, Copy, PartialEq)]
enum DialogResult {
    OpenSettings,
    Quit,
}

/// Current state of required permissions
#[derive(Debug, Clone, Copy)]
pub struct PermissionState {
    pub accessibility: bool,
    pub input_monitoring: bool,
}

impl PermissionState {
    /// Check if all required permissions are granted
    pub fn all_granted(&self) -> bool {
        self.accessibility && self.input_monitoring
    }

    /// Get list of missing permission names
    pub fn missing_permissions(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.accessibility {
            missing.push("Accessibility");
        }
        if !self.input_monitoring {
            missing.push("Input Monitoring");
        }
        missing
    }
}

// ============================================================================
// Permission Checking Functions
// ============================================================================

/// Check if Accessibility permission is granted
pub fn check_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Check if Input Monitoring permission is granted
pub fn check_input_monitoring_permission() -> bool {
    unsafe { CGRequestPostEventAccess() }
}

/// Check all required permissions
pub fn check_all_permissions() -> PermissionState {
    PermissionState {
        accessibility: check_accessibility_permission(),
        input_monitoring: check_input_monitoring_permission(),
    }
}

// ============================================================================
// System Settings Openers
// ============================================================================

/// Open System Settings to Accessibility pane
fn open_accessibility_settings() -> Result<()> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .context("Failed to open Accessibility settings")?;

    log::info!("Opened System Settings → Privacy & Security → Accessibility");
    Ok(())
}

/// Open System Settings to Input Monitoring pane
fn open_input_monitoring_settings() -> Result<()> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn()
        .context("Failed to open Input Monitoring settings")?;

    log::info!("Opened System Settings → Privacy & Security → Input Monitoring");
    Ok(())
}

// ============================================================================
// Permission Dialog Implementation
// ============================================================================

/// Format the informative text for the permission dialog
fn format_permission_message(state: &PermissionState) -> String {
    let mut message = String::from("pthkd needs the following permissions to function:\n\n");

    if !state.accessibility {
        message.push_str("❌ Accessibility\n");
        message.push_str("   Required to monitor keyboard events and control applications\n\n");
    } else {
        message.push_str("✅ Accessibility (granted)\n\n");
    }

    if !state.input_monitoring {
        message.push_str("❌ Input Monitoring\n");
        message.push_str("   Required to capture keystrokes\n\n");
    } else {
        message.push_str("✅ Input Monitoring (granted)\n\n");
    }

    message.push_str("The app cannot start without these permissions.\n\n");
    message.push_str("Click \"Open System Settings\" to grant permissions,\n");
    message.push_str("then enable pthkd in the Privacy & Security settings.");

    message
}

// ============================================================================
// MacOSSession Extensions for Permissions
// ============================================================================

impl MacOSSession {
    /// Show a critical permission dialog
    ///
    /// Creates a native macOS alert dialog with critical styling that blocks
    /// until the user responds. Returns DialogResult indicating choice.
    unsafe fn show_permission_dialog(&self, state: &PermissionState) -> DialogResult {
        log::debug!("Showing permission dialog");

        // Create alert using session method
        let alert_class = match self.get_class("NSAlert") {
            Ok(class) => class,
            Err(e) => {
                log::error!("Failed to get NSAlert class: {}", e);
                return DialogResult::Quit;
            }
        };

        let alert = match self.alloc_init(alert_class) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Failed to create NSAlert: {}", e);
                return DialogResult::Quit;
            }
        };

        // Set alert style to critical (NSAlertStyleCritical = 2)
        let _: () = msg_send![alert, setAlertStyle: 2_i64];

        // Set title and informative text using session methods
        let title = match self.create_nsstring("Permissions Required") {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create title string: {}", e);
                return DialogResult::Quit;
            }
        };

        let message = format_permission_message(state);
        let message_ns = match self.create_nsstring(&message) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create message string: {}", e);
                return DialogResult::Quit;
            }
        };

        let _: () = msg_send![alert, setMessageText: title];
        let _: () = msg_send![alert, setInformativeText: message_ns];

        // Add buttons using session methods (order matters - first button is default/highlighted)
        let open_button = match self.create_nsstring("Open System Settings") {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create button string: {}", e);
                return DialogResult::Quit;
            }
        };

        let quit_button = match self.create_nsstring("Quit") {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create button string: {}", e);
                return DialogResult::Quit;
            }
        };

        let _: *mut AnyObject = msg_send![alert, addButtonWithTitle: open_button];
        let _: *mut AnyObject = msg_send![alert, addButtonWithTitle: quit_button];

        // Show modal dialog - this BLOCKS until user clicks a button
        let response: i64 = msg_send![alert, runModal];

        // NSAlertFirstButtonReturn = 1000, NSAlertSecondButtonReturn = 1001
        match response {
            1000 => {
                log::debug!("User clicked 'Open System Settings'");
                DialogResult::OpenSettings
            }
            1001 => {
                log::debug!("User clicked 'Quit'");
                DialogResult::Quit
            }
            _ => {
                log::warn!("Unexpected dialog response: {}", response);
                DialogResult::Quit
            }
        }
    }
}

/// Show a blocking permission dialog using NSAlert
///
/// Legacy wrapper that calls the session method
unsafe fn show_permission_dialog(state: &PermissionState) -> DialogResult {
    MacOSSession::global().show_permission_dialog(state)
}

// ============================================================================
// Main Permission Workflow
// ============================================================================

/// Ensure all required permissions are granted
///
/// This function implements a blocking loop that:
/// 1. Checks if both Accessibility and Input Monitoring permissions are granted
/// 2. If not, shows a blocking dialog explaining what's needed
/// 3. Allows user to open System Settings to grant permissions
/// 4. Re-checks permissions after user interaction
/// 5. Repeats until all permissions are granted or user quits
///
/// This function will not return until all permissions are granted,
/// or the user explicitly quits the app.
///
/// # Errors
///
/// Returns an error if System Settings cannot be opened.
pub fn ensure_permissions_granted() -> Result<()> {
    log::info!("Checking required permissions...");

    loop {
        let state = check_all_permissions();

        // Log current permission state
        log::debug!(
            "Permission state - Accessibility: {}, Input Monitoring: {}",
            state.accessibility,
            state.input_monitoring
        );

        // If all permissions are granted, we're done!
        if state.all_granted() {
            log::info!("✅ All permissions granted");
            return Ok(());
        }

        // Log which permissions are missing
        let missing = state.missing_permissions();
        log::warn!("Missing permissions: {}", missing.join(", "));

        // Show blocking dialog
        let result = unsafe { show_permission_dialog(&state) };

        match result {
            DialogResult::OpenSettings => {
                // Open appropriate System Settings pane based on what's missing
                if !state.accessibility {
                    open_accessibility_settings()
                        .context("Failed to open Accessibility settings")?;
                } else if !state.input_monitoring {
                    open_input_monitoring_settings()
                        .context("Failed to open Input Monitoring settings")?;
                }

                // Wait a moment for System Settings to open
                thread::sleep(Duration::from_secs(1));

                log::info!("Waiting for user to grant permissions in System Settings...");

                // Loop will re-check permissions on next iteration
            }
            DialogResult::Quit => {
                log::info!("User chose to quit - exiting");
                std::process::exit(0);
            }
        }

        // Small delay before re-checking to avoid tight loop
        // User needs time to navigate System Settings and enable permissions
        thread::sleep(Duration::from_millis(500));
    }
}
