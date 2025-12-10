//! Menu interaction implementation for MacOSSession
//!
//! Functions for clicking menu items via accessibility API

use super::session::MacOSSession;
use super::ffi;
use anyhow::{Result, bail};
use libc::c_void;
use std::ptr;

impl MacOSSession {
    /// Click a menu item by path
    ///
    /// # Example
    /// ```ignore
    /// macos.click_menu_item("Pro Tools", &["File", "Save"]).await?;
    /// ```
    pub async fn click_menu_item(
        &mut self,
        app_name: &str,
        menu_path: &[&str],
    ) -> Result<()> {
        if menu_path.is_empty() {
            bail!("Menu path cannot be empty");
        }

        if !self.has_accessibility_permission() {
            bail!("Accessibility permissions not granted");
        }

        // Use cached PID
        let pid = self.get_pid(app_name)?;

        log::info!("Clicking menu in {}: {:?}", app_name, menu_path);

        unsafe {
            let app_ref = ffi::create_app_element(pid)?;

            // Get the menu bar
            let menu_bar_key = ffi::create_cfstring("AXMenuBar");
            let mut menu_bar_value: *mut c_void = ptr::null_mut();

            let result = ffi::AXUIElementCopyAttributeValue(
                app_ref,
                menu_bar_key,
                &mut menu_bar_value,
            );
            ffi::CFRelease(menu_bar_key);

            if result != ffi::K_AX_ERROR_SUCCESS || menu_bar_value.is_null() {
                ffi::CFRelease(app_ref);
                bail!("Failed to get menu bar (error: {})", result);
            }

            // Navigate through the menu path
            let mut current_element = menu_bar_value;

            for (i, &menu_title) in menu_path.iter().enumerate() {
                log::info!("Looking for menu item: {}", menu_title);

                // Get children
                let children_key = ffi::create_cfstring("AXChildren");
                let mut children_value: *mut c_void = ptr::null_mut();

                let result = ffi::AXUIElementCopyAttributeValue(
                    current_element,
                    children_key,
                    &mut children_value,
                );
                ffi::CFRelease(children_key);

                if result != ffi::K_AX_ERROR_SUCCESS || children_value.is_null() {
                    ffi::CFRelease(menu_bar_value);
                    ffi::CFRelease(app_ref);
                    bail!("Failed to get children at path level {} (error: {})", i, result);
                }

                let count = ffi::CFArrayGetCount(children_value);
                let mut found = false;

                for j in 0..count {
                    let child = ffi::CFArrayGetValueAtIndex(children_value, j) as ffi::AXUIElementRef;

                    // Get title
                    if let Ok(Some(title)) = ffi::get_ax_string_attribute(child, "AXTitle") {
                        if crate::normalize(&title) == crate::normalize(menu_title) {
                            log::info!("Found menu item: {} (matched '{}')", title, menu_title);

                            // Last item in path - click it
                            if i == menu_path.len() - 1 {
                                log::info!("Clicking final menu item: {}", menu_title);
                                let press_action = ffi::create_cfstring("AXPress");
                                let result = ffi::AXUIElementPerformAction(child, press_action);
                                ffi::CFRelease(press_action);

                                ffi::CFRelease(menu_bar_value);
                                ffi::CFRelease(app_ref);

                                if result != ffi::K_AX_ERROR_SUCCESS {
                                    bail!("Failed to click menu item '{}' (error: {})", menu_title, result);
                                }

                                log::info!("✅ Successfully clicked menu item");
                                return Ok(());
                            } else {
                                // For menu items, get the submenu container (first child)
                                let mut submenu_value: *mut c_void = ptr::null_mut();

                                ffi::AXUIElementCopyAttributeValue(
                                    child,
                                    children_key,
                                    &mut submenu_value,
                                );

                                if !submenu_value.is_null() {
                                    let submenu_count = ffi::CFArrayGetCount(submenu_value);
                                    if submenu_count > 0 {
                                        // Get first child - the actual submenu container
                                        current_element = ffi::CFArrayGetValueAtIndex(submenu_value, 0) as ffi::AXUIElementRef;
                                        found = true;
                                        // Note: Don't release submenu_value - it's managed by CF
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                // Note: Don't release children_value - elements from it may still be in use

                if !found && i < menu_path.len() - 1 {
                    ffi::CFRelease(menu_bar_value);
                    ffi::CFRelease(app_ref);
                    bail!("Could not find menu item: {} at level {}", menu_title, i);
                }
            }

            ffi::CFRelease(menu_bar_value);
            ffi::CFRelease(app_ref);

            bail!("Menu navigation completed but item not clicked")
        }
    }
}
