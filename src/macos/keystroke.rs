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
pub fn send_keystroke(keys: &[&str]) -> Result<()> {
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

    unsafe {
        use super::helpers::CGEvent;

        // Create event source (RAII wrapper for auto-cleanup)
        let event_source = CGEvent::new(CGEventSourceCreate(CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE));
        if event_source.is_null() {
            bail!("Failed to create event source");
        }

        // Send key-down events with modifier flags
        for &key_code in &key_codes {
            let key_down_event = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), key_code, true));
            if key_down_event.is_null() {
                bail!("Failed to create key down event for keycode {}", key_code);
            }

            // Set modifier flags if any
            if modifier_flags != 0 {
                CGEventSetFlags(key_down_event.as_ptr(), modifier_flags);
            }

            CGEventPost(CG_HID_EVENT_TAP, key_down_event.as_ptr());
            // key_down_event automatically released here
        }

        // Send key-up events in reverse order with modifier flags
        for &key_code in key_codes.iter().rev() {
            let key_up_event = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), key_code, false));
            if key_up_event.is_null() {
                bail!("Failed to create key up event for keycode {}", key_code);
            }

            // Set modifier flags on key up as well
            if modifier_flags != 0 {
                CGEventSetFlags(key_up_event.as_ptr(), modifier_flags);
            }

            CGEventPost(CG_HID_EVENT_TAP, key_up_event.as_ptr());
            // key_up_event automatically released here
        }

        // event_source automatically released here
    }

    Ok(())
}

/// Map a character to its key name and whether it needs shift
fn char_to_key(ch: char) -> Option<(&'static str, bool)> {
    match ch {
        // Lowercase letters - no shift
        'a'..='z' => Some((
            match ch {
                'a' => "a",
                'b' => "b",
                'c' => "c",
                'd' => "d",
                'e' => "e",
                'f' => "f",
                'g' => "g",
                'h' => "h",
                'i' => "i",
                'j' => "j",
                'k' => "k",
                'l' => "l",
                'm' => "m",
                'n' => "n",
                'o' => "o",
                'p' => "p",
                'q' => "q",
                'r' => "r",
                's' => "s",
                't' => "t",
                'u' => "u",
                'v' => "v",
                'w' => "w",
                'x' => "x",
                'y' => "y",
                'z' => "z",
                _ => unreachable!(),
            },
            false,
        )),

        // Uppercase letters - with shift
        'A'..='Z' => Some((
            match ch {
                'A' => "a",
                'B' => "b",
                'C' => "c",
                'D' => "d",
                'E' => "e",
                'F' => "f",
                'G' => "g",
                'H' => "h",
                'I' => "i",
                'J' => "j",
                'K' => "k",
                'L' => "l",
                'M' => "m",
                'N' => "n",
                'O' => "o",
                'P' => "p",
                'Q' => "q",
                'R' => "r",
                'S' => "s",
                'T' => "t",
                'U' => "u",
                'V' => "v",
                'W' => "w",
                'X' => "x",
                'Y' => "y",
                'Z' => "z",
                _ => unreachable!(),
            },
            true,
        )),

        // Numbers - no shift
        '0' => Some(("0", false)),
        '1' => Some(("1", false)),
        '2' => Some(("2", false)),
        '3' => Some(("3", false)),
        '4' => Some(("4", false)),
        '5' => Some(("5", false)),
        '6' => Some(("6", false)),
        '7' => Some(("7", false)),
        '8' => Some(("8", false)),
        '9' => Some(("9", false)),

        // Shifted number row
        '!' => Some(("1", true)),
        '@' => Some(("2", true)),
        '#' => Some(("3", true)),
        '$' => Some(("4", true)),
        '%' => Some(("5", true)),
        '^' => Some(("6", true)),
        '&' => Some(("7", true)),
        '*' => Some(("8", true)),
        '(' => Some(("9", true)),
        ')' => Some(("0", true)),

        // Punctuation - no shift
        ' ' => Some(("space", false)),
        '-' => Some(("-", false)),
        '=' => Some(("=", false)),
        '[' => Some(("[", false)),
        ']' => Some(("]", false)),
        '\\' => Some(("\\", false)),
        ';' => Some((";", false)),
        '\'' => Some(("'", false)),
        ',' => Some((",", false)),
        '.' => Some((".", false)),
        '/' => Some(("/", false)),
        '`' => Some(("`", false)),

        // Shifted punctuation
        '_' => Some(("-", true)),
        '+' => Some(("=", true)),
        '{' => Some(("[", true)),
        '}' => Some(("]", true)),
        '|' => Some(("\\", true)),
        ':' => Some((";", true)),
        '"' => Some(("'", true)),
        '<' => Some((",", true)),
        '>' => Some((".", true)),
        '?' => Some(("/", true)),
        '~' => Some(("`", true)),

        _ => None,
    }
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
    use crate::keycodes::key_name_to_codes;

    for ch in text.chars() {
        // Map character to key name and shift status
        let (key_name, needs_shift) =
            char_to_key(ch).ok_or_else(|| anyhow::anyhow!("Unsupported character: '{}'", ch))?;

        let key_code = key_name_to_codes(key_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key_name))?[0];

        unsafe {
            use super::helpers::CGEvent;

            // Create event source (RAII wrapper for auto-cleanup)
            let event_source = CGEvent::new(CGEventSourceCreate(CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE));
            if event_source.is_null() {
                bail!("Failed to create event source");
            }

            const APP_MARKER: i64 = 0x5054484B44;
            const EVENT_USER_DATA_FIELD: u32 = 127;

            if needs_shift {
                // Send shift down (keycode 56 = left shift)
                let shift_down = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), 56, true));
                CGEventSetIntegerValueField(shift_down.as_ptr(), EVENT_USER_DATA_FIELD, APP_MARKER);
                CGEventPost(CG_HID_EVENT_TAP, shift_down.as_ptr());
                // shift_down automatically released here
            }

            // Send key down
            let key_down = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), key_code, true));
            CGEventSetIntegerValueField(key_down.as_ptr(), EVENT_USER_DATA_FIELD, APP_MARKER);
            // Set shift flag on the key event (or explicitly clear it)
            CGEventSetFlags(key_down.as_ptr(), if needs_shift { CG_EVENT_FLAG_MASK_SHIFT } else { 0 });
            CGEventPost(CG_HID_EVENT_TAP, key_down.as_ptr());
            // key_down automatically released here

            // Send key up
            let key_up = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), key_code, false));
            CGEventSetIntegerValueField(key_up.as_ptr(), EVENT_USER_DATA_FIELD, APP_MARKER);
            // Set shift flag on key up (or explicitly clear it)
            CGEventSetFlags(key_up.as_ptr(), if needs_shift { CG_EVENT_FLAG_MASK_SHIFT } else { 0 });
            CGEventPost(CG_HID_EVENT_TAP, key_up.as_ptr());
            // key_up automatically released here

            if needs_shift {
                // Send shift up
                let shift_up = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), 56, false));
                CGEventSetIntegerValueField(shift_up.as_ptr(), EVENT_USER_DATA_FIELD, APP_MARKER);
                CGEventPost(CG_HID_EVENT_TAP, shift_up.as_ptr());
                // shift_up automatically released here
            }

            // event_source automatically released here
        }

        // Small delay between characters
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
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
    use crate::keycodes::key_name_to_codes;

    for ch in text.chars() {
        // Map character to key name and shift status
        let (key_name, needs_shift) =
            char_to_key(ch).ok_or_else(|| anyhow::anyhow!("Unsupported character: '{}'", ch))?;

        let key_code = key_name_to_codes(key_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key_name))?[0];

        unsafe {
            use super::helpers::CGEvent;

            // Create event source (RAII wrapper for auto-cleanup)
            let event_source = CGEvent::new(CGEventSourceCreate(CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE));
            if event_source.is_null() {
                bail!("Failed to create event source");
            }

            // NOTE: NO APP_MARKER - these events should look like real keystrokes

            if needs_shift {
                // Send shift down (keycode 56 = left shift)
                let shift_down = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), 56, true));
                CGEventPost(CG_HID_EVENT_TAP, shift_down.as_ptr());
                // shift_down automatically released here
            }

            // Send key down
            let key_down = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), key_code, true));
            // Set shift flag on the key event (or explicitly clear it)
            CGEventSetFlags(key_down.as_ptr(), if needs_shift { CG_EVENT_FLAG_MASK_SHIFT } else { 0 });
            CGEventPost(CG_HID_EVENT_TAP, key_down.as_ptr());
            // key_down automatically released here

            // Send key up
            let key_up = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), key_code, false));
            // Set shift flag on key up (or explicitly clear it)
            CGEventSetFlags(key_up.as_ptr(), if needs_shift { CG_EVENT_FLAG_MASK_SHIFT } else { 0 });
            CGEventPost(CG_HID_EVENT_TAP, key_up.as_ptr());
            // key_up automatically released here

            if needs_shift {
                // Send shift up
                let shift_up = CGEvent::new(CGEventCreateKeyboardEvent(event_source.as_ptr(), 56, false));
                CGEventPost(CG_HID_EVENT_TAP, shift_up.as_ptr());
                // shift_up automatically released here
            }

            // event_source automatically released here
        }

        // Small delay between characters
        std::thread::sleep(std::time::Duration::from_millis(10));
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
