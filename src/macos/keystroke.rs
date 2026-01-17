//! Keystroke sending functionality
//!
//! STATUS: EXPERIMENTAL - Testing in progress
//!
//! Current status:
//! - ✅ Global keystroke sending (send_keystroke) - Using pure C API
//! - 🚧 App-specific keystrokes (send_keystroke_to_app) - Needs main thread dispatch

use anyhow::{Result, bail};
use libc::c_void;

// ============================================================================
// Core Graphics Event FFI
// ============================================================================

const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
const CG_HID_EVENT_TAP: u32 = 0;
const CG_SESSION_EVENT_TAP: u32 = 1;

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
    fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
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
pub fn keystroke(keys: &[&str]) -> Result<()> {
    use crate::keycodes::key_name_to_codes;

    if keys.is_empty() {
        bail!("No keys specified");
    }

    log::debug!("send_keystroke called with keys: {:?}", keys);

    // Separate modifiers from regular keys
    let mut modifier_flags = 0u64;
    let mut regular_keys = Vec::new();

    for &key_name in keys {
        if is_modifier(key_name) {
            // It's a modifier - add to flags
            if let Some(flag) = modifier_to_flag(key_name) {
                modifier_flags |= flag;
                log::debug!("  Added modifier: {} (flag: 0x{:x})", key_name, flag);
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
        let keycode = codes[0];
        key_codes.push(keycode);
        log::debug!("  Key '{}' -> keycode {}", key_name, keycode);
    }

    // Use Swift bridge to send keystroke
    crate::macos::swift_bridge::send_global_keystroke(&key_codes, modifier_flags)
}

/// Type text by sending individual keystrokes for each character
///
/// This converts a string into individual keystroke events.
/// Handles uppercase and special characters by adding shift modifier automatically.
///
/// # Arguments
/// * `text` - The text string to type
///
/// # Example
/// ```ignore
/// type_text("Hello123!@#")?;
/// ```
pub fn type_text(text: &str) -> Result<()> {
    // Use Swift bridge - mark_events=true to prevent event tap from catching
    crate::macos::swift_bridge::type_text(text, true)
}

/// Type text for password fields (without APP_MARKER)
///
/// This variant does NOT mark events with APP_MARKER, allowing them to behave
/// like genuine user keystrokes. This is critical for password fields which may
/// filter or reject programmatically-marked events.
///
/// Safe to use from hotkey callbacks because:
/// - The hotkey that triggered this has already been consumed
/// - User is no longer pressing those keys
/// - No risk of infinite loops
///
/// # Arguments
/// * `text` - The text string to type
///
/// # Example
/// ```ignore
/// type_text_for_password("MyPassword123")?;
/// ```
pub fn type_text_for_password(text: &str) -> Result<()> {
    // Use Swift bridge - mark_events=false to appear as genuine keystrokes
    crate::macos::swift_bridge::type_text(text, false)
}

/// Paste text for password fields using clipboard and Cmd+V
///
/// This is the most reliable method for password fields that filter or reject
/// programmatic keystrokes. Works by temporarily using the clipboard to paste.
///
/// Safe to use from hotkey callbacks because:
/// - The hotkey that triggered this has already been consumed
/// - User is no longer pressing those keys
/// - No risk of infinite loops
/// - Previous clipboard contents are restored automatically
///
/// # Arguments
/// * `text` - The text string to paste
///
/// # Example
/// ```ignore
/// paste_text_for_password("MyPassword123")?;
/// ```
pub fn paste_text_for_password(text: &str) -> Result<()> {
    // Use Swift bridge - uses clipboard to bypass password field restrictions
    crate::macos::swift_bridge::paste_text(text)
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
