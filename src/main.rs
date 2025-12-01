mod actions;
mod config;
mod hotkey;
mod keycodes;
mod macos;
mod protools;

use anyhow::Context;
use config::{config_to_hotkeys, load_config};
use hotkey::{HOTKEYS, KEY_STATE, PENDING_HOTKEY, PendingHotkey};

use libc::c_void;
use std::ptr;
use std::io::Write;

// Event tap callback - tracks key state and checks registered hotkeys
unsafe extern "C" fn key_event_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: *mut c_void,
    _user_info: *mut c_void,
) -> *mut c_void {
    // Get key state (should always be initialized by this point)
    let key_state = KEY_STATE.get().expect("KEY_STATE not initialized");

    if event_type == macos::CG_EVENT_KEY_DOWN {
        let key_code = unsafe {
            macos::CGEventGetIntegerValueField(event, macos::CG_EVENT_FIELD_KEYBOARD_EVENT_KEYCODE)
        } as u16;

        // Update key state
        let mut state = key_state.lock().unwrap();
        state.key_down(key_code);
        drop(state);

        // Check all registered hotkeys against current key state
        if let Some(hotkeys) = HOTKEYS.get() {
            let state = key_state.lock().unwrap();
            let pressed_keys = state.get_pressed_keys();

            for (index, hotkey) in hotkeys.iter().enumerate() {
                if hotkey.matches(pressed_keys) {
                    if hotkey.trigger_on_release {
                        // Mark as pending, trigger on key release
                        let pending = PENDING_HOTKEY.get().expect("PENDING_HOTKEY not initialized");
                        *pending.lock().unwrap() = Some(PendingHotkey {
                            hotkey_index: index,
                            chord_keys: pressed_keys.clone(),
                        });
                        return ptr::null_mut(); // Consume event
                    } else {
                        // Trigger immediately
                        (hotkey.action)();
                        return ptr::null_mut(); // Consume event
                    }
                }
            }
        }
    } else if event_type == macos::CG_EVENT_KEY_UP {
        let key_code = unsafe {
            macos::CGEventGetIntegerValueField(event, macos::CG_EVENT_FIELD_KEYBOARD_EVENT_KEYCODE)
        } as u16;

        // Check if we have a pending hotkey
        let pending_hotkey_guard = PENDING_HOTKEY.get().expect("PENDING_HOTKEY not initialized");
        let pending_opt = pending_hotkey_guard.lock().unwrap().clone();

        // Update key state
        let mut state = key_state.lock().unwrap();
        state.key_up(key_code);
        let pressed_keys = state.get_pressed_keys().clone();
        drop(state);

        // If we have a pending hotkey, check if all chord keys are released
        if let Some(pending) = pending_opt {
            // Check if any of the chord keys are still pressed
            let any_chord_key_pressed = pending.chord_keys.iter().any(|k| pressed_keys.contains(k));

            if !any_chord_key_pressed {
                // All chord keys released - trigger the action!
                // Small delay to let the system fully process key releases
                std::thread::sleep(std::time::Duration::from_millis(50));

                if let Some(hotkeys) = HOTKEYS.get() {
                    if let Some(hotkey) = hotkeys.get(pending.hotkey_index) {
                        (hotkey.action)();
                    }
                }

                // Clear the pending hotkey
                *pending_hotkey_guard.lock().unwrap() = None;
            }
        }
    } else if event_type == macos::CG_EVENT_FLAGS_CHANGED {
        // Modifier key pressed or released
        let key_code = unsafe {
            macos::CGEventGetIntegerValueField(event, macos::CG_EVENT_FIELD_KEYBOARD_EVENT_KEYCODE)
        } as u16;

        let flags = unsafe { macos::CGEventGetFlags(event) };

        // Determine if this is a press or release based on the flags
        use crate::keycodes::*;
        let is_pressed = match key_code {
            KEY_CMD_LEFT | KEY_CMD_RIGHT => (flags & macos::CG_EVENT_FLAG_MASK_COMMAND) != 0,
            KEY_SHIFT_LEFT | KEY_SHIFT_RIGHT => (flags & macos::CG_EVENT_FLAG_MASK_SHIFT) != 0,
            KEY_OPTION_LEFT | KEY_OPTION_RIGHT => (flags & macos::CG_EVENT_FLAG_MASK_ALTERNATE) != 0,
            KEY_CONTROL_LEFT | KEY_CONTROL_RIGHT => (flags & macos::CG_EVENT_FLAG_MASK_CONTROL) != 0,
            _ => return event, // Unknown modifier
        };

        // Update key state
        let mut state = key_state.lock().unwrap();
        if is_pressed {
            state.key_down(key_code);
        } else {
            state.key_up(key_code);
        }
        drop(state);

        // Check hotkeys after modifier change (same logic as KEY_DOWN)
        if is_pressed {
            // Only check for new matches on key down, not release
            if let Some(hotkeys) = HOTKEYS.get() {
                let state = key_state.lock().unwrap();
                let pressed_keys = state.get_pressed_keys();

                for (index, hotkey) in hotkeys.iter().enumerate() {
                    if hotkey.matches(pressed_keys) {
                        if hotkey.trigger_on_release {
                            // Mark as pending, trigger on key release
                            let pending = PENDING_HOTKEY.get().expect("PENDING_HOTKEY not initialized");
                            *pending.lock().unwrap() = Some(PendingHotkey {
                                hotkey_index: index,
                                chord_keys: pressed_keys.clone(),
                            });
                            return ptr::null_mut(); // Consume event
                        } else {
                            // Trigger immediately
                            (hotkey.action)();
                            return ptr::null_mut(); // Consume event
                        }
                    }
                }
            }
        } else {
            // Modifier released - check for pending hotkey trigger (same as KEY_UP)
            let pending_hotkey_guard = PENDING_HOTKEY.get().expect("PENDING_HOTKEY not initialized");
            let pending_opt = pending_hotkey_guard.lock().unwrap().clone();

            if let Some(pending) = pending_opt {
                let state = key_state.lock().unwrap();
                let pressed_keys = state.get_pressed_keys();

                // Check if any of the chord keys are still pressed
                let any_chord_key_pressed = pending.chord_keys.iter().any(|k| pressed_keys.contains(k));

                if !any_chord_key_pressed {
                    // All chord keys released - trigger the action!
                    // Small delay to let the system fully process key releases
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    if let Some(hotkeys) = HOTKEYS.get() {
                        if let Some(hotkey) = hotkeys.get(pending.hotkey_index) {
                            (hotkey.action)();
                        }
                    }

                    // Clear the pending hotkey
                    *pending_hotkey_guard.lock().unwrap() = None;
                }
            }
        }
    }

    // Pass through other events
    event
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    // Initialize logging with file clearing on startup
    init_logging()?;

    log::info!("Starting hotkey daemon...");

    // Initialize ProTools tokio runtime
    protools::init_runtime();

    // Initialize key state tracker
    use hotkey::KeyState;
    use std::sync::Mutex;
    KEY_STATE
        .set(Mutex::new(KeyState::new()))
        .map_err(|_| anyhow::anyhow!("Failed to initialize KEY_STATE - already initialized"))?;

    // Initialize pending hotkey tracker
    PENDING_HOTKEY
        .set(Mutex::new(None))
        .map_err(|_| anyhow::anyhow!("Failed to initialize PENDING_HOTKEY - already initialized"))?;

    // Load configuration from config.toml
    let config = load_config("config.toml")
        .context("Failed to load config.toml - make sure it exists in the current directory")?;

    // Convert config to hotkeys
    let hotkeys = config_to_hotkeys(config)
        .context("Failed to parse config")?;

    // Log registered hotkeys
    log::info!("Registered {} hotkeys:", hotkeys.len());
    for hotkey in &hotkeys {
        log::info!("  - {} => {}", hotkey.chord.describe(), hotkey.action_name);
    }

    // Initialize hotkey registry
    HOTKEYS.set(hotkeys)
        .map_err(|_| anyhow::anyhow!("Failed to initialize hotkeys - already initialized"))?;

    // Create and install event tap for keyboard events
    unsafe {
        let event_tap = macos::create_keyboard_event_tap(key_event_callback)
            .context("Failed to create event tap")?;

        macos::install_event_tap_on_run_loop(event_tap);

        log::info!("Hotkey daemon is running. Listening for hotkeys...");
        macos::run_event_loop();
    }

    Ok(())
}

/// Initialize logging system
/// Note: Log file is cleared on recompile (in build.rs), not on each run
fn init_logging() -> anyhow::Result<()> {
    use std::fs::OpenOptions;

    let log_file_path = "macrod.log";

    // Configure env_logger to write to the file (append mode)
    let target = Box::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)  // Append across multiple runs
            .open(log_file_path)
            .context("Failed to open log file for writing")?
    );

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {:5}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    Ok(())
}
