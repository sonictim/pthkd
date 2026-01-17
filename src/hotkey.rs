use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

// ============================================================================
// Key State Tracking
// ============================================================================

/// Tracks the current state of pressed keys
#[derive(Debug, Clone)]
pub struct KeyState {
    /// Set of currently pressed key codes (Arc for cheap cloning)
    pressed_keys: Arc<HashSet<u16>>,
    // Future: Track press history with timestamps for sequential chord support
    // press_history: VecDeque<(u16, Instant)>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyState {
    pub fn new() -> Self {
        Self {
            pressed_keys: Arc::new(HashSet::new()),
        }
    }

    /// Records a key press
    pub fn key_down(&mut self, key_code: u16) {
        // Clone-on-write: create new HashSet with the key added
        let mut new_set = (*self.pressed_keys).clone();
        new_set.insert(key_code);
        self.pressed_keys = Arc::new(new_set);
    }

    /// Records a key release
    pub fn key_up(&mut self, key_code: u16) {
        // Clone-on-write: create new HashSet with the key removed
        let mut new_set = (*self.pressed_keys).clone();
        new_set.remove(&key_code);
        self.pressed_keys = Arc::new(new_set);
    }

    /// Returns the current set of pressed keys (Arc clone is cheap - just a pointer copy)
    pub fn get_pressed_keys(&self) -> Arc<HashSet<u16>> {
        Arc::clone(&self.pressed_keys)
    }
}

/// Global key state accessible from C callback
pub static KEY_STATE: OnceLock<Mutex<KeyState>> = OnceLock::new();

// ============================================================================
// Chord Pattern System
// ============================================================================

/// Represents different types of chord patterns
#[derive(Debug, Clone)]
pub enum ChordPattern {
    /// Simultaneous chord: all keys must be pressed at the same time
    ///
    /// Each Vec<u16> represents a "key group" where ANY key in the group satisfies it.
    /// ALL groups must be satisfied for the chord to match.
    ///
    /// Example: ["cmd", "shift", "s"] becomes:
    /// - Group 0: [55, 54] (left or right CMD)
    /// - Group 1: [56, 60] (left or right Shift)
    /// - Group 2: [1] (S key)
    ///
    /// Matches when: (55 OR 54) AND (56 OR 60) AND 1 are ALL pressed
    Simultaneous { key_groups: Vec<Vec<u16>> },
    // Future: Sequential chords for multi-tap patterns
    // Sequential { steps: Vec<ChordStep> },
}

impl ChordPattern {
    /// Checks if the current pressed keys match this chord pattern
    pub fn matches(&self, pressed_keys: &HashSet<u16>) -> bool {
        match self {
            ChordPattern::Simultaneous { key_groups } => {
                // Check that ALL key groups are satisfied
                let all_groups_satisfied = key_groups.iter().all(|group| {
                    // At least ONE key from this group must be pressed
                    group.iter().any(|&key| pressed_keys.contains(&key))
                });

                if !all_groups_satisfied {
                    return false;
                }

                // Count how many keys from our chord are actually pressed
                let chord_keys_pressed: usize = key_groups
                    .iter()
                    .map(|group| {
                        // Count how many keys from this group are pressed
                        group
                            .iter()
                            .filter(|&&key| pressed_keys.contains(&key))
                            .count()
                    })
                    .sum();

                // Ensure ONLY our chord keys are pressed (no extra keys)
                // This prevents CMD+Shift+L from matching when CMD+Shift+Option+L is pressed
                pressed_keys.len() == chord_keys_pressed
            }
        }
    }

    /// Returns a human-readable description of the chord for logging
    pub fn describe(&self) -> String {
        use crate::keycodes::keycode_to_name;

        match self {
            ChordPattern::Simultaneous { key_groups } => {
                let parts: Vec<String> = key_groups
                    .iter()
                    .filter_map(|group| {
                        // Get the name of the first key in the group
                        // (for modifiers with L/R variants, they map to the same name anyway)
                        group
                            .first()
                            .and_then(|&code| keycode_to_name(code).map(|name| name.to_string()))
                    })
                    .collect();
                parts.join("+")
            }
        }
    }
}

// ============================================================================
// Trigger Pattern System (Keyboard + MIDI)
// ============================================================================

/// Unified trigger pattern that supports keyboard, MIDI, or hybrid triggers
#[derive(Debug, Clone)]
pub enum TriggerPattern {
    /// Keyboard chord pattern
    Keyboard(ChordPattern),

    /// MIDI pattern
    Midi(crate::input::midi::MidiPattern),

    // Future: Hybrid keyboard + MIDI triggers
    // Hybrid { keyboard: ChordPattern, midi: MidiPattern },
}

impl TriggerPattern {
    /// Returns a human-readable description of the trigger for logging
    pub fn describe(&self) -> String {
        match self {
            TriggerPattern::Keyboard(chord) => chord.describe(),
            TriggerPattern::Midi(pattern) => {
                // Describe MIDI pattern
                match pattern {
                    crate::input::midi::MidiPattern::Simultaneous { messages } => {
                        let parts: Vec<String> = messages.iter().map(|spec| {
                            match spec {
                                crate::input::midi::MidiMessageSpec::Note { note } => {
                                    format!("note{}", note)
                                }
                                crate::input::midi::MidiMessageSpec::ControlChange { cc } => {
                                    format!("cc{}", cc)
                                }
                            }
                        }).collect();
                        parts.join("+")
                    }
                }
            }
        }
    }
}

// ============================================================================
// Hotkey Definition
// ============================================================================

/// Represents a hotkey binding
#[derive(Debug)]
pub struct Hotkey {
    /// The trigger pattern to match (keyboard or MIDI)
    pub trigger: TriggerPattern,

    /// The action name (for logging)
    pub action_name: String,

    /// The action function to execute
    pub action: fn(&crate::params::Params) -> anyhow::Result<()>,

    /// Parameters to pass to the action function
    pub params: crate::params::Params,

    /// Whether to trigger on key release instead of key down
    pub trigger_on_release: bool,

    /// Whether to show notification on action completion
    pub notify: bool,

    /// Whether to register as a Carbon hotkey (works during secure input)
    pub carbon: bool,

    /// Whether to check if user is in a text field before triggering (prevents accidental triggers while typing)
    pub check_for_text_field: bool,

    /// Target applications (hotkey only fires when one of these apps is focused)
    pub application: Option<Vec<String>>,

    pub app_window: Option<String>,
}

impl Hotkey {
    /// Checks if this hotkey's keyboard chord matches the current key state
    pub fn matches_keyboard(&self, pressed_keys: &HashSet<u16>) -> bool {
        // Check if trigger is keyboard type
        let trigger_matches = match &self.trigger {
            TriggerPattern::Keyboard(chord) => chord.matches(pressed_keys),
            _ => false, // Not a keyboard trigger
        };

        trigger_matches && self.check_application_filters()
    }

    /// Checks if this hotkey's MIDI pattern matches the current MIDI state
    pub fn matches_midi(&self, active_midi: &HashSet<crate::input::midi::MidiMessage>) -> bool {
        // Check if trigger is MIDI type
        let trigger_matches = match &self.trigger {
            TriggerPattern::Midi(pattern) => pattern.matches(active_midi),
            _ => false, // Not a MIDI trigger
        };

        trigger_matches && self.check_application_filters()
    }

    /// Check application and window filters (shared by keyboard and MIDI)
    fn check_application_filters(&self) -> bool {
        (self.application.is_none()
            || match (
                &self.application,
                crate::macos::app_info::get_current_app().ok(),
            ) {
                (Some(config_apps), Some(current_app)) => {
                    // Check if any of the configured apps match the current app
                    config_apps
                        .iter()
                        .any(|app| crate::soft_match(&current_app, app))
                }
                _ => false,
            })
            && match &self.app_window {
                None => true,
                Some(config_window) => match crate::macos::app_info::get_app_window().ok() {
                    None => false,
                    Some(app_window) => crate::soft_match(&app_window, config_window),
                },
            }
    }
}

/// Global hotkey registry accessible from C callback
pub static HOTKEYS: OnceLock<Mutex<Vec<Hotkey>>> = OnceLock::new();

// ============================================================================
// Pending Hotkey Tracking (for trigger_on_release)
// ============================================================================

/// Tracks a hotkey that matched but is waiting for key release to trigger
#[derive(Debug, Clone)]
pub struct PendingHotkey {
    /// Index into HOTKEYS array
    pub hotkey_index: usize,

    /// The keys that were part of the matched chord (Arc for cheap cloning)
    pub chord_keys: Arc<HashSet<u16>>,
}

/// Global pending hotkey state
pub static PENDING_HOTKEY: OnceLock<Mutex<Option<PendingHotkey>>> = OnceLock::new();

use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use std::future::Future;

pub struct HotkeyCounter {
    count: u32,
    last_call: Instant,
    last_timeout: Duration,
    pending_task: Option<JoinHandle<()>>,
}

impl HotkeyCounter {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_call: Instant::now() - Duration::from_secs(10), // Far in the past
            last_timeout: Duration::from_millis(500), // Default timeout
            pending_task: None,
        }
    }

    /// Register a keypress and schedule delayed execution
    ///
    /// Each call cancels the previous pending execution and starts a new timer.
    /// When the timer expires, the callback is invoked with the final press count.
    ///
    /// # Arguments
    /// * `timeout_ms` - Timeout in milliseconds before executing the callback
    /// * `max` - Maximum number of presses to cycle through (e.g., 3 means cycle 0,1,2,0,1,2...)
    /// * `callback` - Async function to execute after timeout, receives the final press count (0-based)
    pub fn press<F, Fut>(&mut self, timeout_ms: u64, max: u32, callback: F)
    where
        F: FnOnce(u32) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Cancel any pending execution
        if let Some(handle) = self.pending_task.take() {
            handle.abort();
            log::info!("Aborted previous pending task");
        }

        let now = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        // Reset count if timeout has passed since last press
        if now.duration_since(self.last_call) > self.last_timeout {
            log::info!("Timeout passed, resetting count from {} to 0", self.count);
            self.count = 0;
        }

        self.count += 1;
        self.last_call = now;
        self.last_timeout = timeout;

        // Return 0-based index for array indexing
        let final_count = (self.count - 1) % max;

        log::info!("HotkeyCounter: count={}, max={}, final_count={}, will execute in {}ms",
                   self.count, max, final_count, timeout_ms);

        // Spawn delayed execution task
        let handle = tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            callback(final_count).await;
        });

        self.pending_task = Some(handle);
    }
}
