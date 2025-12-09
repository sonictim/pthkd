//! Keystroke sending implementation for MacOSSession
//!
//! Functions for sending global keystrokes

use super::session::MacOSSession;
use super::ffi;
use anyhow::{Result, bail};

impl MacOSSession {
    /// Send a global keystroke with optional modifiers
    ///
    /// # Example
    /// ```ignore
    /// macos.send_keystroke(0x7D, &["cmd"]).await?; // Cmd+Down
    /// ```
    pub async fn send_keystroke(&mut self, keycode: u16, modifiers: &[&str]) -> Result<()> {
        unsafe {
            // Create event source
            let source = ffi::CGEventSourceCreate(ffi::CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE);
            if source.is_null() {
                bail!("Failed to create CGEventSource");
            }

            // Calculate modifier flags
            let mut flags: u64 = 0;
            for &modifier in modifiers {
                flags |= match modifier.to_lowercase().as_str() {
                    "cmd" | "command" => ffi::CG_EVENT_FLAG_MASK_COMMAND,
                    "shift" => ffi::CG_EVENT_FLAG_MASK_SHIFT,
                    "option" | "alt" => ffi::CG_EVENT_FLAG_MASK_ALTERNATE,
                    "control" | "ctrl" => ffi::CG_EVENT_FLAG_MASK_CONTROL,
                    _ => {
                        ffi::CFRelease(source);
                        bail!("Unknown modifier: {}", modifier);
                    }
                };
            }

            // Create and post key down event
            let key_down = ffi::CGEventCreateKeyboardEvent(source, keycode, true);
            if key_down.is_null() {
                ffi::CFRelease(source);
                bail!("Failed to create key down event");
            }
            ffi::CGEventSetFlags(key_down, flags);
            ffi::CGEventPost(ffi::CG_HID_EVENT_TAP, key_down);
            ffi::CFRelease(key_down);

            // Small delay between down and up
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Create and post key up event
            let key_up = ffi::CGEventCreateKeyboardEvent(source, keycode, false);
            if key_up.is_null() {
                ffi::CFRelease(source);
                bail!("Failed to create key up event");
            }
            ffi::CGEventSetFlags(key_up, flags);
            ffi::CGEventPost(ffi::CG_HID_EVENT_TAP, key_up);
            ffi::CFRelease(key_up);

            ffi::CFRelease(source);

            log::info!("✅ Sent keystroke: keycode={}, modifiers={:?}", keycode, modifiers);
            Ok(())
        }
    }
}
