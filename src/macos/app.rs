//! App control implementation for MacOSSession
//!
//! Functions for focusing apps, getting current app, etc.

use super::session::MacOSSession;
use super::ffi;
use anyhow::{Result, bail};
use objc2::{class, msg_send};
use objc2::runtime::AnyObject;
use libc::c_void;
use std::ptr;

impl MacOSSession {
    /// Focus an application by name
    ///
    /// This is a simplified version that handles the most common case
    pub async fn focus_app(&mut self, app_name: &str) -> Result<()> {
        log::info!("Attempting to focus '{}'...", app_name);

        // Get PID (uses cache if available)
        let pid = self.get_pid(app_name)?;

        unsafe {
            // Use NSWorkspace to activate the app
            let workspace_class = class!(NSWorkspace);
            let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
            let running_apps: *mut AnyObject = msg_send![workspace, runningApplications];
            let count: usize = msg_send![running_apps, count];

            for i in 0..count {
                let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
                let app_pid: i32 = msg_send![app, processIdentifier];

                if app_pid == pid {
                    // Found the app, activate it
                    let success: bool = msg_send![app, activateWithOptions: 0];
                    if success {
                        log::info!("✅ '{}' is now focused", app_name);
                        return Ok(());
                    } else {
                        bail!("Failed to activate '{}'", app_name);
                    }
                }
            }

            bail!("App '{}' found but couldn't be activated", app_name)
        }
    }

    /// Get name of currently focused app
    pub async fn get_focused_app(&mut self) -> Result<String> {
        unsafe {
            let workspace_class = class!(NSWorkspace);
            let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
            let frontmost_app: *mut AnyObject = msg_send![workspace, frontmostApplication];

            if frontmost_app.is_null() {
                bail!("No frontmost application found");
            }

            let name_ns: *mut c_void = msg_send![frontmost_app, localizedName];
            if let Some(name) = ffi::cfstring_to_string(name_ns) {
                Ok(name)
            } else {
                bail!("Could not get frontmost app name");
            }
        }
    }

    /// Get all running application names
    pub async fn get_running_apps(&mut self) -> Result<Vec<String>> {
        unsafe {
            let workspace_class = class!(NSWorkspace);
            let workspace: *mut AnyObject = msg_send![workspace_class, sharedWorkspace];
            let running_apps: *mut AnyObject = msg_send![workspace, runningApplications];
            let count: usize = msg_send![running_apps, count];

            let mut apps = Vec::new();

            for i in 0..count {
                let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
                let name_ns: *mut c_void = msg_send![app, localizedName];

                if let Some(name) = ffi::cfstring_to_string(name_ns) {
                    apps.push(name);
                }
            }

            Ok(apps)
        }
    }

    /// Check if accessibility permissions are granted
    pub fn has_accessibility_permission(&self) -> bool {
        unsafe { ffi::AXIsProcessTrusted() }
    }

    /// Get focused window name of currently focused app
    pub async fn get_focused_window(&mut self) -> Result<String> {
        unsafe {
            let system_wide = ffi::AXUIElementCreateSystemWide();
            if system_wide.is_null() {
                bail!("Failed to create system-wide accessibility element");
            }

            // Get focused app element
            let focused_app_key = ffi::create_cfstring("AXFocusedApplication");
            let mut focused_app: *mut c_void = ptr::null_mut();
            let result = ffi::AXUIElementCopyAttributeValue(
                system_wide,
                focused_app_key,
                &mut focused_app,
            );
            ffi::CFRelease(focused_app_key);
            ffi::CFRelease(system_wide);

            if result != ffi::K_AX_ERROR_SUCCESS || focused_app.is_null() {
                bail!("Failed to get focused application");
            }

            // Get focused window of app
            let focused_window_key = ffi::create_cfstring("AXFocusedWindow");
            let mut focused_window: *mut c_void = ptr::null_mut();
            let result = ffi::AXUIElementCopyAttributeValue(
                focused_app as ffi::AXUIElementRef,
                focused_window_key,
                &mut focused_window,
            );
            ffi::CFRelease(focused_window_key);
            ffi::CFRelease(focused_app);

            if result != ffi::K_AX_ERROR_SUCCESS || focused_window.is_null() {
                bail!("Failed to get focused window");
            }

            // Get window title
            let title = ffi::get_ax_string_attribute(
                focused_window as ffi::AXUIElementRef,
                "AXTitle",
            )?;
            ffi::CFRelease(focused_window);

            title.ok_or_else(|| anyhow::anyhow!("Window has no title"))
        }
    }

    /// Check if currently in a text field
    pub async fn is_in_text_field(&mut self) -> Result<bool> {
        unsafe {
            let system_wide = ffi::AXUIElementCreateSystemWide();
            if system_wide.is_null() {
                bail!("Failed to create system-wide accessibility element");
            }

            // Get focused element
            let focused_key = ffi::create_cfstring("AXFocusedUIElement");
            let mut focused_element: *mut c_void = ptr::null_mut();
            let result = ffi::AXUIElementCopyAttributeValue(
                system_wide,
                focused_key,
                &mut focused_element,
            );
            ffi::CFRelease(focused_key);
            ffi::CFRelease(system_wide);

            if result != ffi::K_AX_ERROR_SUCCESS || focused_element.is_null() {
                return Ok(false);
            }

            // Get role
            let role = ffi::get_ax_string_attribute(
                focused_element as ffi::AXUIElementRef,
                "AXRole",
            )?;
            ffi::CFRelease(focused_element);

            Ok(matches!(
                role.as_deref(),
                Some("AXTextField") | Some("AXTextArea") | Some("AXComboBox")
            ))
        }
    }
}
