//! Keystroke sending functionality
//!
//! STATUS: EXPERIMENTAL - Testing in progress
//!
//! Current status:
//! - ✅ Global keystroke sending (send_keystroke) - Using pure C API
//! - 🚧 App-specific keystrokes (send_keystroke_to_app) - Needs main thread dispatch

use anyhow::{Result, bail};
use libc::c_void;
use super::ffi::CFRelease;

// ============================================================================
// Core Graphics Event FFI
// ============================================================================

const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
const CG_HID_EVENT_TAP: u32 = 0;

// Modifier key flags
const CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
const CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
const CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000; // Option key
const CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;

unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;

    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CGEventSourceCreate(source_state_id: i32) -> *mut c_void;
}

// ============================================================================
// Modifier Helpers
// ============================================================================

/// Check if a key name is a modifier
fn is_modifier(key_name: &str) -> bool {
    matches!(
        key_name.to_lowercase().as_str(),
        "cmd" | "command" | "shift" | "option" | "alt" | "control" | "ctrl"
    )
}

/// Convert modifier key name to flag
fn modifier_to_flag(key_name: &str) -> Option<u64> {
    match key_name.to_lowercase().as_str() {
        "cmd" | "command" => Some(CG_EVENT_FLAG_MASK_COMMAND),
        "shift" => Some(CG_EVENT_FLAG_MASK_SHIFT),
        "option" | "alt" => Some(CG_EVENT_FLAG_MASK_ALTERNATE),
        "control" | "ctrl" => Some(CG_EVENT_FLAG_MASK_CONTROL),
        _ => None,
    }
}

// ============================================================================
// Global Keystroke Sending
// ============================================================================

/// Send a global keystroke chord
///
/// **STATUS: WORKING** - Uses pure C Core Graphics API
///
/// This posts keyboard events to the system event queue. The keystrokes
/// go to whatever application currently has focus.
///
/// Modifiers (cmd, shift, option, control) are applied as flags on the
/// regular key events, not sent as separate key events. This is the proper
/// way macOS applications expect to receive modified keystrokes.
///
/// # Arguments
/// * `keys` - Slice of key names to press simultaneously (e.g., &["cmd", "f1"])
///
/// # Example
/// ```ignore
/// // Send Cmd+S
/// send_keystroke(&["cmd", "s"])?;
///
/// // Send just Space
/// send_keystroke(&["space"])?;
///
/// // Send Cmd+Shift+F1
/// send_keystroke(&["cmd", "shift", "f1"])?;
/// ```
pub fn send_keystroke(keys: &[&str]) -> Result<()> {
    use crate::keycodes::key_name_to_codes;

    if keys.is_empty() {
        bail!("No keys specified");
    }

    // Separate modifiers from regular keys
    let mut modifier_flags = 0u64;
    let mut regular_keys = Vec::new();

    for &key_name in keys {
        if is_modifier(key_name) {
            // It's a modifier - add to flags
            if let Some(flag) = modifier_to_flag(key_name) {
                modifier_flags |= flag;
            }
        } else {
            // It's a regular key - add to list
            regular_keys.push(key_name);
        }
    }

    // Must have at least one regular key
    if regular_keys.is_empty() {
        bail!("Must specify at least one non-modifier key");
    }

    // Parse regular key names to keycodes
    let mut key_codes = Vec::new();
    for key_name in regular_keys {
        let codes = key_name_to_codes(key_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown key name: {}", key_name))?;

        // For keys with multiple options, use the first one
        key_codes.push(codes[0]);
    }

    unsafe {
        // Create event source
        let event_source = CGEventSourceCreate(CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE);
        if event_source.is_null() {
            bail!("Failed to create event source");
        }

        // Send key-down events with modifier flags
        for &key_code in &key_codes {
            let key_down_event = CGEventCreateKeyboardEvent(event_source, key_code, true);
            if key_down_event.is_null() {
                CFRelease(event_source);
                bail!("Failed to create key down event for keycode {}", key_code);
            }

            // Set modifier flags if any
            if modifier_flags != 0 {
                CGEventSetFlags(key_down_event, modifier_flags);
            }

            CGEventPost(CG_HID_EVENT_TAP, key_down_event);
            CFRelease(key_down_event);
        }

        // Send key-up events in reverse order with modifier flags
        for &key_code in key_codes.iter().rev() {
            let key_up_event = CGEventCreateKeyboardEvent(event_source, key_code, false);
            if key_up_event.is_null() {
                CFRelease(event_source);
                bail!("Failed to create key up event for keycode {}", key_code);
            }

            // Set modifier flags on key up as well
            if modifier_flags != 0 {
                CGEventSetFlags(key_up_event, modifier_flags);
            }

            CGEventPost(CG_HID_EVENT_TAP, key_up_event);
            CFRelease(key_up_event);
        }

        // Clean up
        CFRelease(event_source);
    }

    Ok(())
}

// ============================================================================
// App-Specific Keystroke Sending
// ============================================================================

/// Send a keystroke to a specific application
///
/// **STATUS: NOT IMPLEMENTED**
///
/// This function requires:
/// 1. App focusing via NSRunningApplication (Objective-C)
/// 2. Main thread dispatch mechanism
/// 3. Proper exception handling
///
/// For now, use `send_keystroke()` and manually focus the app.
pub fn send_keystroke_to_app(_app_name: &str, _keys: &[&str]) -> Result<()> {
    bail!(
        "send_keystroke_to_app not yet implemented - use send_keystroke() with manual app focus for now"
    )
}
