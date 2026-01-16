//! Carbon hotkey registration for secure input scenarios
//!
//! Carbon hotkeys (RegisterEventHotKey) continue working when CGEventTap
//! is disabled by macOS during secure input (password fields, sudo, etc.)
//!
//! This module provides a complementary hotkey system alongside CGEventTap.
//! Hotkeys marked with `carbon = true` in config.toml will be registered
//! using Carbon in addition to (or instead of) CGEventTap.

use anyhow::Result;
use libc::c_void;
use std::collections::HashMap;
use std::ptr;
use std::sync::Mutex;

// ============================================================================
// Carbon Event Manager FFI
// ============================================================================

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {}

#[repr(C)]
struct EventHotKeyID {
    signature: u32, // OSType - 4 chars like 'pthk'
    id: u32,        // Unique ID for this hotkey
}

type EventHandlerRef = *mut c_void;
type EventHandlerCallRef = *mut c_void;
type EventRef = *mut c_void;
type EventTargetRef = *mut c_void;
type EventHotKeyRef = *mut c_void;

#[allow(non_snake_case)]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> EventTargetRef;

    fn InstallEventHandler(
        target: EventTargetRef,
        handler: unsafe extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> i32,
        num_types: u32,
        type_list: *const EventTypeSpec,
        user_data: *mut c_void,
        out_ref: *mut EventHandlerRef,
    ) -> i32;

    fn RegisterEventHotKey(
        key_code: u32,
        modifiers: u32,
        hotkey_id: EventHotKeyID,
        target: EventTargetRef,
        options: u32,
        out_ref: *mut EventHotKeyRef,
    ) -> i32;

    fn UnregisterEventHotKey(hotkey: EventHotKeyRef) -> i32;

    fn GetEventParameter(
        event: EventRef,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        buffer_size: u32,
        actual_size: *mut u32,
        data: *mut c_void,
    ) -> i32;
}

#[repr(C)]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}

// Carbon event constants
const K_EVENT_CLASS_KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
const K_EVENT_PARAM_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");
const TYPE_EVENT_HOT_KEY_ID: u32 = u32::from_be_bytes(*b"hkid");

// Modifier key constants for Carbon (different from CGEvent!)
const CMD_KEY: u32 = 1 << 8; // cmdKey
const SHIFT_KEY: u32 = 1 << 9; // shiftKey
const OPTION_KEY: u32 = 1 << 11; // optionKey
const CONTROL_KEY: u32 = 1 << 12; // controlKey

// ============================================================================
// Global State
// ============================================================================

/// Send-safe wrapper for raw pointers (we ensure they're only used on main thread)
struct SendPtr(*mut c_void);
unsafe impl Send for SendPtr {}

/// Maps Carbon hotkey ID to index in global HOTKEYS array
static CARBON_HOTKEY_MAP: Mutex<Option<HashMap<u32, usize>>> = Mutex::new(None);

/// Stores EventHotKeyRef pointers for cleanup
static CARBON_HOTKEY_REFS: Mutex<Vec<SendPtr>> = Mutex::new(Vec::new());

/// Event handler reference for cleanup
static CARBON_EVENT_HANDLER: Mutex<Option<SendPtr>> = Mutex::new(None);

// ============================================================================
// Event Handler Callback
// ============================================================================

unsafe extern "C" fn carbon_hotkey_handler(
    _call_ref: EventHandlerCallRef,
    event: EventRef,
    _user_data: *mut c_void,
) -> i32 {
    // Wrap entire callback in catch_unwind to prevent panics from crossing FFI boundary
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        // Extract the hotkey ID from the event
        let mut hotkey_id = EventHotKeyID {
            signature: 0,
            id: 0,
        };

        let status = GetEventParameter(
            event,
            K_EVENT_PARAM_DIRECT_OBJECT,
            TYPE_EVENT_HOT_KEY_ID,
            ptr::null_mut(),
            std::mem::size_of::<EventHotKeyID>() as u32,
            ptr::null_mut(),
            &mut hotkey_id as *mut _ as *mut c_void,
        );

        if status != 0 {
            eprintln!("Carbon: Failed to get hotkey ID: status {}", status);
            return status;
        }

        eprintln!("🔥 Carbon hotkey triggered: ID {}", hotkey_id.id);

        // Look up the hotkey in our map
        let map_guard = match CARBON_HOTKEY_MAP.lock() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("Carbon: Failed to lock hotkey map: {}", e);
                return -1;
            }
        };

        if let Some(map) = map_guard.as_ref() {
            if let Some(&hotkey_index) = map.get(&hotkey_id.id) {
                drop(map_guard); // Release lock before calling into hotkey system

                eprintln!("🔥 Triggering hotkey by index {} from Carbon", hotkey_index);

                // Trigger the hotkey action through the existing system
                crate::trigger_hotkey_by_index(hotkey_index);

                eprintln!("✅ Carbon hotkey action completed");
            } else {
                eprintln!("Carbon: Hotkey ID {} not found in map", hotkey_id.id);
            }
        } else {
            eprintln!("Carbon: Hotkey map not initialized!");
        }

        0 // noErr
    }));

    match result {
        Ok(status) => status,
        Err(_) => {
            eprintln!("💥 PANIC in carbon_hotkey_handler!");
            -1 // Return error status
        }
    }
}

// ============================================================================
// Registration
// ============================================================================

/// Registers Carbon hotkeys for all hotkeys marked with `carbon = true`
///
/// Call this after initializing HOTKEYS but before running the event loop
pub fn register_carbon_hotkeys() -> Result<()> {
    // Wrap in catch_unwind to prevent panics from aborting
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        let hotkeys_guard = match crate::hotkey::HOTKEYS.get() {
            Some(h) => h,
            None => {
                eprintln!("Carbon: HOTKEYS not initialized");
                return Err(anyhow::anyhow!("HOTKEYS not initialized"));
            }
        };

        let hotkeys_guard = match hotkeys_guard.lock() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Carbon: Failed to lock HOTKEYS: {}", e);
                return Err(anyhow::anyhow!("Failed to lock HOTKEYS: {}", e));
            }
        };

        let hotkeys = &*hotkeys_guard;

        // Initialize the hotkey map
        let mut map = HashMap::new();
        let mut refs = Vec::new();

        // Install event handler (once for all hotkeys)
        let event_types = [EventTypeSpec {
            event_class: K_EVENT_CLASS_KEYBOARD,
            event_kind: K_EVENT_HOT_KEY_PRESSED,
        }];

        let mut handler_ref: EventHandlerRef = ptr::null_mut();
        let target = GetApplicationEventTarget();

        if target.is_null() {
            eprintln!("Carbon: GetApplicationEventTarget returned null");
            return Err(anyhow::anyhow!("GetApplicationEventTarget returned null"));
        }

        let status = InstallEventHandler(
            target,
            carbon_hotkey_handler,
            event_types.len() as u32,
            event_types.as_ptr(),
            ptr::null_mut(),
            &mut handler_ref,
        );

        if status != 0 {
            eprintln!("Carbon: Failed to install event handler: status {}", status);
            return Err(anyhow::anyhow!(
                "Failed to install Carbon event handler: status {}",
                status
            ));
        }

        match CARBON_EVENT_HANDLER.lock() {
            Ok(mut guard) => *guard = Some(SendPtr(handler_ref)),
            Err(e) => {
                eprintln!("Carbon: Failed to store handler ref: {}", e);
                return Err(anyhow::anyhow!("Failed to store handler ref: {}", e));
            }
        }

        eprintln!("✓ Carbon event handler installed");

        // Register each hotkey that has carbon = true
        let mut hotkey_id = 1u32; // Start IDs at 1

        for (index, hotkey) in hotkeys.iter().enumerate() {
            // Skip hotkeys not marked for Carbon registration
            if !hotkey.carbon {
                continue;
            }

            // Convert chord to Carbon hotkey spec
            if let Some((key_code, modifiers)) = chord_to_carbon_spec(&hotkey.chord) {
                let hk_id = EventHotKeyID {
                    signature: u32::from_be_bytes(*b"pthk"), // 'pthk' signature
                    id: hotkey_id,
                };

                let mut hotkey_ref: EventHotKeyRef = ptr::null_mut();

                let status = RegisterEventHotKey(
                    key_code,
                    modifiers,
                    hk_id,
                    target,
                    0, // options
                    &mut hotkey_ref,
                );

                if status == 0 {
                    map.insert(hotkey_id, index);
                    refs.push(SendPtr(hotkey_ref));

                    eprintln!(
                        "✓ Registered Carbon hotkey #{}: {} (keycode={}, mods=0x{:x})",
                        hotkey_id, hotkey.action_name, key_code, modifiers
                    );

                    hotkey_id += 1;
                } else {
                    eprintln!(
                        "✗ Failed to register Carbon hotkey for '{}': status {}",
                        hotkey.action_name, status
                    );
                }
            } else {
                eprintln!(
                    "✗ Skipping Carbon registration for '{}' - not compatible (complex chord)",
                    hotkey.action_name
                );
            }
        }

        match CARBON_HOTKEY_MAP.lock() {
            Ok(mut guard) => *guard = Some(map),
            Err(e) => {
                eprintln!("Carbon: Failed to store hotkey map: {}", e);
                return Err(anyhow::anyhow!("Failed to store hotkey map: {}", e));
            }
        }

        match CARBON_HOTKEY_REFS.lock() {
            Ok(mut guard) => *guard = refs,
            Err(e) => {
                eprintln!("Carbon: Failed to store hotkey refs: {}", e);
                return Err(anyhow::anyhow!("Failed to store hotkey refs: {}", e));
            }
        }

        let registered_count = hotkey_id - 1;
        if registered_count > 0 {
            eprintln!(
                "✅ Registered {} Carbon hotkey(s) (will work during secure input)",
                registered_count
            );
        } else {
            eprintln!("⚠️  No hotkeys marked with carbon=true");
        }

        Ok(())
    }));

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            eprintln!("💥 PANIC in register_carbon_hotkeys!");
            Err(anyhow::anyhow!("Panic in register_carbon_hotkeys"))
        }
    }
}

/// Converts a ChordPattern to Carbon hotkey specification (keycode + modifiers)
///
/// Returns None if the chord is too complex for Carbon (which only supports
/// simple modifier+key combinations, not OR groups)
fn chord_to_carbon_spec(chord: &crate::hotkey::ChordPattern) -> Option<(u32, u32)> {
    use crate::hotkey::ChordPattern;
    use crate::keycodes::*;

    match chord {
        ChordPattern::Simultaneous { key_groups } => {
            // Carbon can only handle: modifiers + one key
            // We need exactly one non-modifier key and 0+ modifier keys

            let mut key_code: Option<u16> = None;
            let mut modifiers = 0u32;

            for group in key_groups {
                if group.is_empty() {
                    continue;
                }

                // Check if this group is a modifier group (left/right variants)
                // Modifier groups have 2 keycodes (left and right), regular keys have 1
                let first_key = group[0];

                let is_modifier_group = match first_key {
                    KEY_CMD_LEFT | KEY_CMD_RIGHT => {
                        modifiers |= CMD_KEY;
                        true
                    }
                    KEY_SHIFT_LEFT | KEY_SHIFT_RIGHT => {
                        modifiers |= SHIFT_KEY;
                        true
                    }
                    KEY_OPTION_LEFT | KEY_OPTION_RIGHT => {
                        modifiers |= OPTION_KEY;
                        true
                    }
                    KEY_CONTROL_LEFT | KEY_CONTROL_RIGHT => {
                        modifiers |= CONTROL_KEY;
                        true
                    }
                    _ => false,
                };

                if !is_modifier_group {
                    // Non-modifier key - must have exactly one keycode
                    if group.len() != 1 {
                        // This is an OR group (multiple options) - Carbon can't handle it
                        return None;
                    }

                    if key_code.is_some() {
                        // Multiple non-modifier keys - can't handle
                        return None;
                    }

                    key_code = Some(first_key);
                }
            }

            // Must have exactly one non-modifier key
            key_code.map(|kc| (kc as u32, modifiers))
        }
    }
}

/// Unregisters all Carbon hotkeys (for cleanup)
#[allow(dead_code)]
pub fn unregister_carbon_hotkeys() {
    unsafe {
        if let Ok(refs) = CARBON_HOTKEY_REFS.lock() {
            for send_ptr in refs.iter() {
                let _ = UnregisterEventHotKey(send_ptr.0);
            }
        }

        if let Ok(mut map) = CARBON_HOTKEY_MAP.lock() {
            *map = None;
        }

        if let Ok(mut handler) = CARBON_EVENT_HANDLER.lock() {
            *handler = None;
        }

        eprintln!("Unregistered all Carbon hotkeys");
    }
}

// ============================================================================
// Secure Input Detection
// ============================================================================

#[allow(non_snake_case)]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

/// Returns true if secure input is currently enabled
pub fn is_secure_input_active() -> bool {
    unsafe { IsSecureEventInputEnabled() }
}
