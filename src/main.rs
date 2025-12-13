mod async_runtime;
mod config;
mod hotkey;
mod keycodes;
mod macos;
mod params;
mod protools;
mod soundminer;

use anyhow::Context;
use config::{config_to_hotkeys, load_config};
use hotkey::{HOTKEYS, KEY_STATE, PENDING_HOTKEY, PendingHotkey};

use libc::c_void;
use std::io::Write;
use std::ptr;
use std::sync::Arc;

// ============================================================================
// Action Registration Macros
// ============================================================================

/// Unified action macro for both ProTools and macOS actions
///
/// Creates sync wrapper functions that spawn async commands on the shared runtime
///
/// Usage for ProTools:
/// ```ignore
/// actions!("pt", {
///     solo_selected_tracks,
///     go_to_next_marker,
/// });
/// ```
///
/// Usage for macOS:
/// ```ignore
/// actions!("os", {
///     test_notification,
///     focus_protools,
/// });
/// ```
#[macro_export]
macro_rules! actions {
    // ProTools-style (takes ProtoolsSession)
    ("pt", { $($action_name:ident),* $(,)? }) => {
        $(
            pub fn $action_name(params: &$crate::params::Params) -> anyhow::Result<()> {
                use tokio::sync::oneshot;
                let params = params.clone();
                let (tx, rx) = oneshot::channel();

                $crate::async_runtime::spawn_action(async move {
                    let result = async {
                        let mut pt = $crate::protools::ProtoolsSession::new().await?;
                        $crate::protools::commands::$action_name(&mut pt, &params).await
                    }.await;

                    if let Err(ref e) = result {
                        log::error!("ProTools action '{}' failed: {:#}", stringify!($action_name), e);
                    }

                    let _ = tx.send(result);
                });

                // Wait for result (for notification support)
                // This allows notify=true to work correctly
                match $crate::async_runtime::runtime().block_on(async {
                    tokio::time::timeout(
                        tokio::time::Duration::from_secs(10),
                        rx
                    ).await
                }) {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Ok(()), // Channel closed, action still running
                    Err(_) => {
                        log::warn!("Action timed out after 10 seconds");
                        Ok(())
                    }
                }
            }
        )*

        pub fn get_action_registry() -> std::collections::HashMap<&'static str, fn(&$crate::params::Params) -> anyhow::Result<()>> {
            let mut registry = std::collections::HashMap::new();
            $(
                registry.insert(stringify!($action_name), $action_name as fn(&$crate::params::Params) -> anyhow::Result<()>);
            )*
            registry
        }
    };

    // MacOS-style (takes MacOSSession)
    ("os", { $($action_name:ident),* $(,)? }) => {
        $(
            pub fn $action_name(params: &$crate::params::Params) -> anyhow::Result<()> {
                use tokio::sync::oneshot;
                let params = params.clone();
                let (tx, rx) = oneshot::channel();

                $crate::async_runtime::spawn_action(async move {
                    let result = async {
                        let mut macos = $crate::macos::MacOSSession::new()?;
                        $crate::macos::commands::$action_name(&mut macos, &params).await
                    }.await;

                    if let Err(ref e) = result {
                        log::error!("macOS action '{}' failed: {:#}", stringify!($action_name), e);
                    }

                    let _ = tx.send(result);
                });

                // Wait for result (for notification support)
                // This allows notify=true to work correctly
                match $crate::async_runtime::runtime().block_on(async {
                    tokio::time::timeout(
                        tokio::time::Duration::from_secs(10),
                        rx
                    ).await
                }) {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Ok(()), // Channel closed, action still running
                    Err(_) => {
                        log::warn!("Action '{}' timed out after 10 seconds", stringify!($action_name));
                        Ok(())
                    }
                }
            }
        )*

        pub fn get_action_registry() -> std::collections::HashMap<&'static str, fn(&$crate::params::Params) -> anyhow::Result<()>> {
            let mut registry = std::collections::HashMap::new();
            $(
                registry.insert(stringify!($action_name), $action_name as fn(&$crate::params::Params) -> anyhow::Result<()>);
            )*
            registry
        }
    };

    // Simple sync style (no session needed, just calls commands directly)
    ("sync", { $($action_name:ident),* $(,)? }) => {
        $(
            pub fn $action_name(params: &$crate::params::Params) -> anyhow::Result<()> {
                super::commands::$action_name(params)
            }
        )*

        pub fn get_action_registry() -> std::collections::HashMap<&'static str, fn(&$crate::params::Params) -> anyhow::Result<()>> {
            let mut registry = std::collections::HashMap::new();
            $(
                registry.insert(stringify!($action_name), $action_name as fn(&$crate::params::Params) -> anyhow::Result<()>);
            )*
            registry
        }
    };
}

// ============================================================================
// Hotkey Checking Helpers
// ============================================================================

/// Check if any registered hotkey matches the current pressed keys and trigger/queue it
///
/// Returns true if a hotkey was matched and the event should be consumed
fn check_and_trigger_hotkey(pressed_keys: &Arc<std::collections::HashSet<u16>>) -> bool {
    if let Some(hotkeys_mutex) = HOTKEYS.get() {
        let hotkeys = hotkeys_mutex.lock().unwrap();

        for (index, hotkey) in hotkeys.iter().enumerate() {
            if hotkey.matches(&**pressed_keys) {
                if hotkey.trigger_on_release {
                    // Mark as pending, trigger on key release
                    let pending = PENDING_HOTKEY
                        .get()
                        .expect("PENDING_HOTKEY not initialized");
                    *pending.lock().unwrap() = Some(PendingHotkey {
                        hotkey_index: index,
                        chord_keys: Arc::clone(&pressed_keys),
                    });
                    return true; // Consume event
                } else {
                    // Clone action, params, notify, and action_name before dropping lock to avoid deadlock
                    let action = hotkey.action;
                    let params = hotkey.params.clone();
                    let notify = hotkey.notify;
                    let action_name = hotkey.action_name.clone();
                    drop(hotkeys); // Explicitly drop the lock before calling action

                    // Spawn action execution in a separate thread so event tap returns immediately
                    // This prevents macOS from disabling the event tap due to slow callback
                    std::thread::spawn(move || {
                        let result = action(&params);

                        // Show notification if requested
                        if notify {
                            match result {
                                Ok(_) => macos::show_notification(&format!("✅ {}", action_name)),
                                Err(e) => macos::show_notification(&format!("❌ {}: {}", action_name, e)),
                            }
                        }
                    });

                    return true; // Consume event
                }
            }
        }
    }
    false
}

/// Check if a pending hotkey should be triggered (all chord keys released)
///
/// Returns true if a hotkey was triggered
fn check_pending_hotkey_release(pressed_keys: &Arc<std::collections::HashSet<u16>>) -> bool {
    let pending_hotkey_guard = PENDING_HOTKEY
        .get()
        .expect("PENDING_HOTKEY not initialized");
    let pending_opt = pending_hotkey_guard.lock().unwrap().clone();

    if let Some(pending) = pending_opt {
        // Check if any of the chord keys are still pressed
        let any_chord_key_pressed = pending.chord_keys.iter().any(|k| pressed_keys.contains(k));

        if !any_chord_key_pressed {
            // All chord keys released - trigger the action!
            // Small delay to let the system fully process key releases
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Clone action data before dropping lock to avoid deadlock
            let action_data = if let Some(hotkeys_mutex) = HOTKEYS.get() {
                let hotkeys = hotkeys_mutex.lock().unwrap();
                hotkeys.get(pending.hotkey_index).map(|hotkey| {
                    (hotkey.action, hotkey.params.clone(), hotkey.notify, hotkey.action_name.clone())
                })
            } else {
                None
            };

            // Clear the pending hotkey
            *pending_hotkey_guard.lock().unwrap() = None;

            // Spawn action execution in a separate thread so event tap returns immediately
            if let Some((action, params, notify, action_name)) = action_data {
                std::thread::spawn(move || {
                    log::info!("Executing action: {}, notify={}", action_name, notify);
                    let result = action(&params);
                    log::info!("Action '{}' returned: {:?}", action_name, result.is_ok());

                    // Show notification if requested
                    if notify {
                        log::info!("Notification requested for action '{}'", action_name);
                        match result {
                            Ok(_) => {
                                let msg = format!("✅ {}", action_name);
                                log::info!("Showing success notification: {}", msg);
                                macos::show_notification(&msg);
                            }
                            Err(e) => {
                                let msg = format!("❌ {}: {}", action_name, e);
                                log::error!("Showing error notification: {}", msg);
                                macos::show_notification(&msg);
                            }
                        }
                        log::info!("Notification call completed");
                    }
                });
            }

            return true;
        }
    }
    false
}

// ============================================================================
// Event Tap Callback
// ============================================================================

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
        let pressed_keys = state.get_pressed_keys();
        drop(state);

        // Check all registered hotkeys against current key state
        if check_and_trigger_hotkey(&pressed_keys) {
            return ptr::null_mut(); // Consume event
        }
    } else if event_type == macos::CG_EVENT_KEY_UP {
        let key_code = unsafe {
            macos::CGEventGetIntegerValueField(event, macos::CG_EVENT_FIELD_KEYBOARD_EVENT_KEYCODE)
        } as u16;

        // Update key state
        let mut state = key_state.lock().unwrap();
        state.key_up(key_code);
        let pressed_keys = state.get_pressed_keys();
        drop(state);

        // Check if pending hotkey should be triggered
        check_pending_hotkey_release(&pressed_keys);
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
            KEY_OPTION_LEFT | KEY_OPTION_RIGHT => {
                (flags & macos::CG_EVENT_FLAG_MASK_ALTERNATE) != 0
            }
            KEY_CONTROL_LEFT | KEY_CONTROL_RIGHT => {
                (flags & macos::CG_EVENT_FLAG_MASK_CONTROL) != 0
            }
            _ => return event, // Unknown modifier
        };

        // Update key state
        let mut state = key_state.lock().unwrap();
        if is_pressed {
            state.key_down(key_code);
        } else {
            state.key_up(key_code);
        }
        let pressed_keys = state.get_pressed_keys();
        drop(state);

        // Check hotkeys after modifier change
        if is_pressed {
            // Only check for new matches on key down, not release
            if check_and_trigger_hotkey(&pressed_keys) {
                return ptr::null_mut(); // Consume event
            }
        } else {
            // Modifier released - check for pending hotkey trigger
            check_pending_hotkey_release(&pressed_keys);
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
    let log_path = init_logging()?;

    log::info!("===========================================");
    log::info!("Starting macrod hotkey daemon...");
    log::info!("Log file: {}", log_path);
    log::info!("===========================================");

    // Initialize shared async runtime (used by both ProTools and macOS)
    async_runtime::init();

    // Initialize key state tracker
    use hotkey::KeyState;
    use std::sync::Mutex;
    KEY_STATE
        .set(Mutex::new(KeyState::new()))
        .map_err(|_| anyhow::anyhow!("Failed to initialize KEY_STATE - already initialized"))?;

    // Initialize pending hotkey tracker
    PENDING_HOTKEY.set(Mutex::new(None)).map_err(|_| {
        anyhow::anyhow!("Failed to initialize PENDING_HOTKEY - already initialized")
    })?;

    // Load configuration from config.toml
    let config = load_config("config.toml")
        .context("Failed to load config.toml - make sure it exists in the current directory")?;

    // Convert config to hotkeys
    let hotkeys = config_to_hotkeys(config).context("Failed to parse config")?;

    // Log registered hotkeys
    log::info!("Registered {} hotkeys:", hotkeys.len());
    for hotkey in &hotkeys {
        log::info!("  - {} => {}", hotkey.chord.describe(), hotkey.action_name);
    }

    // Initialize hotkey registry
    HOTKEYS
        .set(Mutex::new(hotkeys))
        .map_err(|_| anyhow::anyhow!("Failed to initialize hotkeys - already initialized"))?;

    // Setup menu bar app (must be done before event loop)
    unsafe {
        macos::menu_bar::setup_menu_bar_app()
            .context("Failed to setup menu bar app")?;
    }

    // Create and install event tap for keyboard events
    unsafe {
        let event_tap = macos::create_keyboard_event_tap(key_event_callback)
            .context("Failed to create event tap")?;

        macos::install_event_tap_on_run_loop(event_tap);

        log::info!("Hotkey daemon is running. Listening for hotkeys...");

        // Run event loop (no need to restart - async actions don't block it)
        macos::run_event_loop();
    }

    #[allow(unreachable_code)]
    Ok(())
}

/// Initialize logging system
/// Note: Log file is cleared on recompile (in build.rs), not on each run
/// Returns the absolute path to the log file
fn init_logging() -> anyhow::Result<String> {
    use std::fs::OpenOptions;
    use std::env;

    let log_file_path = "macrod.log";

    // Get absolute path for logging
    let absolute_path = env::current_dir()
        .context("Failed to get current directory")?
        .join(log_file_path)
        .to_string_lossy()
        .to_string();

    // Configure env_logger to write to the file (append mode)
    let target = Box::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true) // Append across multiple runs
            .open(log_file_path)
            .context("Failed to open log file for writing")?,
    );

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info) // Default to Info level if RUST_LOG not set
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

    Ok(absolute_path)
}
/// Normalize a string for comparison: remove whitespace and lowercase
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase()) // handles Unicode correctly
        .collect()
}

/// Soft string matching: case-insensitive, whitespace-insensitive, with partial matching
pub fn soft_match(s1: &str, s2: &str) -> bool {
    let s1 = normalize(s1);
    let s2 = normalize(s2);

    s1 == s2 || s1.contains(&s2) || s2.contains(&s1)
}
