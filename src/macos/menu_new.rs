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
            let mut elements_to_release = vec![app_ref, menu_bar_value];

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
                    for elem in elements_to_release {
                        ffi::CFRelease(elem);
                    }
                    bail!("Failed to get children at path level {} (error: {})", i, result);
                }

                let count = ffi::CFArrayGetCount(children_value);
                let mut found = false;

                for j in 0..count {
                    let child = ffi::CFArrayGetValueAtIndex(children_value, j) as ffi::AXUIElementRef;

                    // Get title
                    if let Ok(Some(title)) = ffi::get_ax_string_attribute(child, "AXTitle") {
                        if crate::soft_match(&title, menu_title) {
                            log::info!("Found menu item: {}", title);

                            // Last item in path - click it
                            if i == menu_path.len() - 1 {
                                let press_action = ffi::create_cfstring("AXPress");
                                let result = ffi::AXUIElementPerformAction(child, press_action);
                                ffi::CFRelease(press_action);

                                ffi::CFRelease(children_value);
                                for elem in elements_to_release {
                                    ffi::CFRelease(elem);
                                }

                                if result != ffi::K_AX_ERROR_SUCCESS {
                                    bail!("Failed to click menu item '{}' (error: {})", menu_title, result);
                                }

                                log::info!("✅ Clicked menu item: {:?}", menu_path);
                                return Ok(());
                            } else {
                                // Not the last item - navigate deeper
                                current_element = child;
                                found = true;
                                break;
                            }
                        }
                    }
                }

                ffi::CFRelease(children_value);

                if !found {
                    for elem in elements_to_release {
                        ffi::CFRelease(elem);
                    }
                    bail!("Menu item '{}' not found at path level {}", menu_title, i);
                }
            }

            for elem in elements_to_release {
                ffi::CFRelease(elem);
            }

            Ok(())
        }
    }
}
