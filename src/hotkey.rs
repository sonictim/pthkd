use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

// ============================================================================
// Key State Tracking
// ============================================================================

/// Tracks the current state of pressed keys
#[derive(Debug, Default)]
pub struct KeyState {
    /// Set of currently pressed key codes
    pub pressed_keys: HashSet<u16>,
    // Future: Track press history with timestamps for sequential chord support
    // press_history: VecDeque<(u16, Instant)>,
}

impl KeyState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    /// Records a key press
    pub fn key_down(&mut self, key_code: u16) {
        self.pressed_keys.insert(key_code);
    }

    /// Records a key release
    pub fn key_up(&mut self, key_code: u16) {
        self.pressed_keys.remove(&key_code);
    }

    /// Returns the current set of pressed keys
    pub fn get_pressed_keys(&self) -> &HashSet<u16> {
        &self.pressed_keys
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
// Hotkey Definition
// ============================================================================

/// Represents a hotkey binding
#[derive(Debug)]
pub struct Hotkey {
    /// The chord pattern to match
    pub chord: ChordPattern,

    /// The action name (for logging)
    pub action_name: String,

    /// The action function to execute
    pub action: fn(),

    /// Whether to trigger on key release instead of key down
    pub trigger_on_release: bool,

    pub application: Option<String>,

    pub app_window: Option<String>,
}

impl Hotkey {
    /// Checks if this hotkey's chord matches the current key state
    pub fn matches(&self, pressed_keys: &HashSet<u16>) -> bool {
        self.chord.matches(pressed_keys)
            && (self.application.is_none()
                || self.application.as_ref() == crate::macos::app_info::get_current_app().ok().as_ref())
            && (self.app_window.is_none()
                || self.app_window.as_ref() == crate::macos::app_info::get_app_window().ok().as_ref())
    }
}

/// Global hotkey registry accessible from C callback
pub static HOTKEYS: OnceLock<Vec<Hotkey>> = OnceLock::new();

// ============================================================================
// Pending Hotkey Tracking (for trigger_on_release)
// ============================================================================

/// Tracks a hotkey that matched but is waiting for key release to trigger
#[derive(Debug, Clone)]
pub struct PendingHotkey {
    /// Index into HOTKEYS array
    pub hotkey_index: usize,

    /// The keys that were part of the matched chord
    pub chord_keys: HashSet<u16>,
}

/// Global pending hotkey state
pub static PENDING_HOTKEY: OnceLock<Mutex<Option<PendingHotkey>>> = OnceLock::new();
