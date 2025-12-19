//! Core keyboard event tap functionality
//!
//! This module handles:
//! - Event tap creation and management
//! - Keyboard event capture
//! - Run loop integration
//!
//! STATUS: STABLE - This code is tested and working

use anyhow::Result;
use libc::c_void;
use std::ptr;
use std::sync::Mutex;

// ============================================================================
// Global Event Tap Storage
// ============================================================================

/// Stores the event tap pointer and callback for monitoring and recreation
struct EventTapState {
    tap: *mut c_void,
    callback: Option<unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void>,
}

unsafe impl Send for EventTapState {}

static EVENT_TAP_STATE: Mutex<Option<EventTapState>> = Mutex::new(None);

// ============================================================================
// Framework Linking
// ============================================================================

#[allow(clippy::duplicated_attributes)]
#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "AppKit", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "System")]
unsafe extern "C" {}

// ============================================================================
// Constants
// ============================================================================

#[allow(dead_code)]
mod constants {
    // Event types
    pub const CG_EVENT_KEY_DOWN: u32 = 10;
    pub const CG_EVENT_KEY_UP: u32 = 11;
    pub const CG_EVENT_FLAGS_CHANGED: u32 = 12;

    // Event fields
    pub const CG_EVENT_FIELD_KEYBOARD_EVENT_KEYCODE: u32 = 9;

    // Event tap locations
    pub const CG_SESSION_EVENT_TAP: u32 = 1;

    // Event tap placements
    pub const CG_HEAD_INSERT_EVENT_TAP: u32 = 0;

    // Event tap options
    pub const CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

    // Event source state
    pub const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;

    // Event posting locations
    pub const CG_HID_EVENT_TAP: u32 = 0;

    // Modifier key flags (same as CGEventFlags)
    pub const CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
    pub const CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;
    pub const CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000; // Option key
    pub const CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
    pub const CG_EVENT_FLAG_MASK_SECONDARY_FN: u64 = 0x00800000; // Fn key
}

// Re-export commonly used constants
pub use constants::*;

// ============================================================================
// Core Graphics Event FFI
// ============================================================================

unsafe extern "C" {
    pub fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    pub fn CGEventGetFlags(event: *mut c_void) -> u64;

    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
        refcon: *mut c_void,
    ) -> *mut c_void;

    pub fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    pub fn CGEventTapIsEnabled(tap: *mut c_void) -> bool;
}

// ============================================================================
// Core Foundation FFI
// ============================================================================

unsafe extern "C" {
    pub fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: *mut c_void,
        order: i64,
    ) -> *mut c_void;

    pub fn CFRunLoopGetCurrent() -> *mut c_void;
    pub fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *mut c_void);
    pub fn CFRunLoopRun();

    pub static kCFRunLoopCommonModes: *mut c_void;
}

// ============================================================================
// Event Tap Management
// ============================================================================

/// Creates a keyboard event tap with the provided callback
///
/// # Safety
/// The callback must be safe to call from the event tap thread
pub unsafe fn create_keyboard_event_tap(
    callback: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
) -> Result<*mut c_void> {
    unsafe {
        // Event mask for key down, key up, and flags changed (for modifiers)
        let event_mask = (1 << CG_EVENT_KEY_DOWN) | (1 << CG_EVENT_KEY_UP) | (1 << CG_EVENT_FLAGS_CHANGED);

        let event_tap = CGEventTapCreate(
            CG_SESSION_EVENT_TAP,
            CG_HEAD_INSERT_EVENT_TAP,
            CG_EVENT_TAP_OPTION_DEFAULT,
            event_mask,
            callback,
            ptr::null_mut(),
        );

        if event_tap.is_null() {
            anyhow::bail!(
                "Failed to create event tap despite permissions check. \
                 This may indicate a system-level issue. \
                 Try restarting the app or your Mac."
            );
        }

        Ok(event_tap)
    }
}

/// Installs an event tap on the current run loop and stores it for monitoring
///
/// # Safety
/// Must be called with a valid event tap pointer
pub unsafe fn install_event_tap_on_run_loop(
    event_tap: *mut c_void,
    callback: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
) {
    unsafe {
        CGEventTapEnable(event_tap, true);

        let run_loop_source = CFMachPortCreateRunLoopSource(ptr::null_mut(), event_tap, 0);
        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, run_loop_source, kCFRunLoopCommonModes);

        // Store the event tap and callback for monitoring
        *EVENT_TAP_STATE.lock().unwrap() = Some(EventTapState {
            tap: event_tap,
            callback: Some(callback),
        });
    }
}

/// Runs the current run loop (blocks forever)
///
/// # Safety
/// This function never returns under normal circumstances
pub unsafe fn run_event_loop() {
    unsafe {
        CFRunLoopRun();
    }
}

// ============================================================================
// Event Tap Monitoring & Recreation
// ============================================================================

/// Checks if the event tap is currently enabled
///
/// Returns true if enabled, false if disabled or not created
pub fn is_event_tap_enabled() -> bool {
    unsafe {
        if let Some(state) = EVENT_TAP_STATE.lock().unwrap().as_ref() {
            if !state.tap.is_null() {
                return CGEventTapIsEnabled(state.tap);
            }
        }
        false
    }
}

/// Recreates the event tap if it has been disabled
///
/// Returns true if recreation was needed and successful, false if tap was already enabled
pub fn recreate_event_tap_if_needed() -> Result<bool> {
    unsafe {
        let state_guard = EVENT_TAP_STATE.lock().unwrap();

        if let Some(state) = state_guard.as_ref() {
            if state.tap.is_null() {
                log::warn!("Event tap pointer is null, cannot recreate");
                return Ok(false);
            }

            // Check if tap is still enabled
            if CGEventTapIsEnabled(state.tap) {
                return Ok(false); // Already enabled, no recreation needed
            }

            log::warn!("Event tap has been disabled by macOS - attempting to recreate");

            // Get the callback before dropping the state
            let callback = state.callback.ok_or_else(|| {
                anyhow::anyhow!("Event tap callback not stored, cannot recreate")
            })?;

            // Drop the old state
            drop(state_guard);

            // Create new event tap
            let new_tap = create_keyboard_event_tap(callback)?;

            // Install it on the run loop (this will update the global state)
            install_event_tap_on_run_loop(new_tap, callback);

            log::info!("Event tap successfully recreated");

            Ok(true)
        } else {
            log::warn!("Event tap state not initialized, cannot recreate");
            Ok(false)
        }
    }
}

/// Checks event tap status and logs if it's been disabled
///
/// This is a lightweight check that can be called frequently
pub fn check_event_tap_status() {
    if !is_event_tap_enabled() {
        log::warn!("Event tap check: tap is currently DISABLED");
    }
}
