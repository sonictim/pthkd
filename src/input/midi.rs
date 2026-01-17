use anyhow::{bail, Result};
use log::{error, info, warn};
use midir::{MidiInput, MidiInputConnection};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::time::sleep;

/// Represents a MIDI message that can trigger hotkeys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MidiMessage {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ControlChange { cc: u8, value: u8 },
}

/// Specification for matching MIDI messages in hotkey patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MidiMessageSpec {
    Note { note: u8 },
    ControlChange { cc: u8 },
    // Future: velocity ranges, etc.
}

impl MidiMessageSpec {
    /// Check if this spec matches a given MIDI message
    pub fn matches(&self, msg: &MidiMessage) -> bool {
        match (self, msg) {
            (Self::Note { note: spec_note }, MidiMessage::NoteOn { note: msg_note, .. }) => {
                spec_note == msg_note
            }
            (Self::ControlChange { cc: spec_cc }, MidiMessage::ControlChange { cc: msg_cc, .. }) => {
                spec_cc == msg_cc
            }
            _ => false,
        }
    }
}

/// MIDI pattern for matching simultaneous MIDI messages (chords)
#[derive(Debug, Clone)]
pub enum MidiPattern {
    Simultaneous { messages: Vec<MidiMessageSpec> },
}

impl MidiPattern {
    /// Check if this pattern matches the current set of active MIDI messages
    pub fn matches(&self, active: &HashSet<MidiMessage>) -> bool {
        match self {
            MidiPattern::Simultaneous { messages } => {
                // All required messages must be active
                let all_present = messages.iter().all(|spec| {
                    active.iter().any(|msg| spec.matches(msg))
                });

                // Exact match: no extra messages allowed
                let exact_count = messages.len() == active.len();

                all_present && exact_count
            }
        }
    }
}

/// Tracks currently active MIDI messages (notes held, recent CCs)
#[derive(Debug, Clone)]
pub struct MidiState {
    active_notes: Arc<HashSet<u8>>,
    active_ccs: Arc<HashMap<u8, u8>>, // CC number -> value
}

impl MidiState {
    pub fn new() -> Self {
        Self {
            active_notes: Arc::new(HashSet::new()),
            active_ccs: Arc::new(HashMap::new()),
        }
    }

    /// Register a note-on event
    pub fn note_on(&mut self, note: u8, _velocity: u8) {
        let mut new_notes = (*self.active_notes).clone();
        new_notes.insert(note);
        self.active_notes = Arc::new(new_notes);
    }

    /// Register a note-off event
    pub fn note_off(&mut self, note: u8) {
        let mut new_notes = (*self.active_notes).clone();
        new_notes.remove(&note);
        self.active_notes = Arc::new(new_notes);
    }

    /// Register a control change event (auto-releases after 50ms)
    pub fn cc(&mut self, cc: u8, value: u8) {
        let mut new_ccs = (*self.active_ccs).clone();
        new_ccs.insert(cc, value);
        self.active_ccs = Arc::new(new_ccs);

        // Schedule CC auto-release after 50ms
        let cc_to_remove = cc;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            if let Some(state) = MIDI_STATE.get() {
                if let Ok(mut s) = state.lock() {
                    let mut new_ccs = (*s.active_ccs).clone();
                    new_ccs.remove(&cc_to_remove);
                    s.active_ccs = Arc::new(new_ccs);
                }
            }
        });
    }

    /// Get all currently active MIDI messages
    pub fn get_active_messages(&self) -> Arc<HashSet<MidiMessage>> {
        let mut messages = HashSet::new();

        // Add active notes
        for &note in self.active_notes.iter() {
            // We don't track velocity in the active state for matching purposes
            // Any velocity > 0 counts as "on"
            messages.insert(MidiMessage::NoteOn { note, velocity: 64 });
        }

        // Add active CCs
        for (&cc, &value) in self.active_ccs.iter() {
            messages.insert(MidiMessage::ControlChange { cc, value });
        }

        Arc::new(messages)
    }
}

/// Global MIDI state (parallel to KEY_STATE)
pub static MIDI_STATE: OnceLock<Mutex<MidiState>> = OnceLock::new();

/// Parse a MIDI spec string like "cc34" or "note60"
pub fn parse_midi_spec(spec: &str) -> Result<MidiMessageSpec> {
    if let Some(num_str) = spec.strip_prefix("cc") {
        let cc = num_str.parse::<u8>()
            .map_err(|_| anyhow::anyhow!("Invalid CC number: {}", num_str))?;
        if cc > 127 {
            bail!("CC number must be 0-127, got {}", cc);
        }
        Ok(MidiMessageSpec::ControlChange { cc })
    } else if let Some(num_str) = spec.strip_prefix("note") {
        let note = num_str.parse::<u8>()
            .map_err(|_| anyhow::anyhow!("Invalid note number: {}", num_str))?;
        if note > 127 {
            bail!("Note number must be 0-127, got {}", note);
        }
        Ok(MidiMessageSpec::Note { note })
    } else {
        bail!("MIDI spec must start with 'cc' or 'note', got: {}", spec)
    }
}

/// Parse MIDI pattern from config (e.g., ["cc34", "note60"])
pub fn parse_midi_pattern(specs: Vec<String>) -> Result<MidiPattern> {
    let messages = specs
        .iter()
        .map(|s| parse_midi_spec(s))
        .collect::<Result<Vec<_>>>()?;

    if messages.is_empty() {
        bail!("MIDI pattern cannot be empty");
    }

    Ok(MidiPattern::Simultaneous { messages })
}

/// Parse raw MIDI bytes into MidiMessage
fn parse_raw_midi(data: &[u8]) -> Option<MidiMessage> {
    if data.len() < 2 {
        return None;
    }

    let status = data[0];
    let data1 = data[1];
    let data2 = if data.len() >= 3 { data[2] } else { 0 };

    // Extract message type and channel
    let msg_type = status & 0xF0;
    let _channel = status & 0x0F;

    match msg_type {
        0x90 => {
            // Note On
            if data2 == 0 {
                // Velocity 0 is treated as Note Off
                Some(MidiMessage::NoteOff { note: data1 })
            } else {
                Some(MidiMessage::NoteOn {
                    note: data1,
                    velocity: data2,
                })
            }
        }
        0x80 => {
            // Note Off
            Some(MidiMessage::NoteOff { note: data1 })
        }
        0xB0 => {
            // Control Change
            Some(MidiMessage::ControlChange {
                cc: data1,
                value: data2,
            })
        }
        _ => None, // Ignore other message types for now
    }
}

/// Connection holder to keep MIDI input alive
static MIDI_CONNECTION: OnceLock<Mutex<Option<MidiInputConnection<()>>>> = OnceLock::new();

/// Initialize MIDI input and start listening
pub fn init_midi_input<F>(
    port_name: Option<&str>,
    channel_filter: Option<u8>,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(MidiMessage) + Send + 'static,
{
    let midi_in = match MidiInput::new("pthkd") {
        Ok(m) => m,
        Err(e) => {
            warn!("MIDI not available: {:#}", e);
            warn!("MIDI hotkeys will not work. Connect MIDI device and reload config.");
            return Ok(()); // Non-fatal, just disable MIDI
        }
    };

    let ports = midi_in.ports();
    if ports.is_empty() {
        warn!("No MIDI input ports found. MIDI hotkeys disabled.");
        warn!("Connect a MIDI device and reload the config to enable MIDI.");
        return Ok(());
    }

    // Select port
    let port = if let Some(name) = port_name {
        // Find specific port by name
        ports.iter().find(|p| {
            midi_in.port_name(p)
                .map(|n| n.contains(name))
                .unwrap_or(false)
        }).ok_or_else(|| anyhow::anyhow!("MIDI port '{}' not found", name))?
    } else {
        // Use first available port
        &ports[0]
    };

    let port_name_str = midi_in.port_name(port).unwrap_or_else(|_| "Unknown".to_string());
    info!("Connecting to MIDI port: {}", port_name_str);

    // Connect with callback
    let connection = midi_in.connect(
        port,
        "pthkd-input",
        move |_timestamp, data, _| {
            if let Some(msg) = parse_raw_midi(data) {
                // Apply channel filter if specified
                if let Some(_filter_channel) = channel_filter {
                    // TODO: Filter by channel (extract from status byte)
                    // For now, accept all channels
                }

                callback(msg);
            }
        },
        (),
    )?;

    // Store connection to keep it alive
    MIDI_CONNECTION.set(Mutex::new(Some(connection)))
        .map_err(|_| anyhow::anyhow!("MIDI connection already initialized"))?;

    info!("MIDI input initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_state_note_tracking() {
        let mut state = MidiState::new();

        state.note_on(60, 100);
        assert!(state.active_notes.contains(&60));

        state.note_off(60);
        assert!(!state.active_notes.contains(&60));
    }

    #[test]
    fn test_midi_pattern_exact_match() {
        let pattern = MidiPattern::Simultaneous {
            messages: vec![
                MidiMessageSpec::Note { note: 60 },
                MidiMessageSpec::Note { note: 64 },
            ],
        };

        let mut msgs = HashSet::new();
        msgs.insert(MidiMessage::NoteOn { note: 60, velocity: 100 });
        msgs.insert(MidiMessage::NoteOn { note: 64, velocity: 100 });
        assert!(pattern.matches(&msgs));

        // Extra note should fail (exact match required)
        msgs.insert(MidiMessage::NoteOn { note: 67, velocity: 100 });
        assert!(!pattern.matches(&msgs));
    }

    #[test]
    fn test_midi_pattern_single_note() {
        let pattern = MidiPattern::Simultaneous {
            messages: vec![MidiMessageSpec::Note { note: 60 }],
        };

        let mut msgs = HashSet::new();
        msgs.insert(MidiMessage::NoteOn { note: 60, velocity: 100 });
        assert!(pattern.matches(&msgs));

        // Should NOT match if extra notes are pressed
        msgs.insert(MidiMessage::NoteOn { note: 64, velocity: 100 });
        assert!(!pattern.matches(&msgs));
    }

    #[test]
    fn test_parse_midi_spec() {
        assert!(matches!(
            parse_midi_spec("cc34").unwrap(),
            MidiMessageSpec::ControlChange { cc: 34 }
        ));

        assert!(matches!(
            parse_midi_spec("note60").unwrap(),
            MidiMessageSpec::Note { note: 60 }
        ));

        assert!(parse_midi_spec("cc128").is_err()); // Out of range
        assert!(parse_midi_spec("note200").is_err()); // Out of range
        assert!(parse_midi_spec("invalid").is_err()); // Invalid format
    }

    #[test]
    fn test_parse_raw_midi() {
        // Note On (channel 1)
        let msg = parse_raw_midi(&[0x90, 60, 100]).unwrap();
        assert_eq!(msg, MidiMessage::NoteOn { note: 60, velocity: 100 });

        // Note Off (channel 1)
        let msg = parse_raw_midi(&[0x80, 60, 0]).unwrap();
        assert_eq!(msg, MidiMessage::NoteOff { note: 60 });

        // Note On with velocity 0 (treated as Note Off)
        let msg = parse_raw_midi(&[0x90, 60, 0]).unwrap();
        assert_eq!(msg, MidiMessage::NoteOff { note: 60 });

        // Control Change (channel 1)
        let msg = parse_raw_midi(&[0xB0, 34, 127]).unwrap();
        assert_eq!(msg, MidiMessage::ControlChange { cc: 34, value: 127 });
    }

    #[test]
    fn test_midi_message_spec_matches() {
        let note_spec = MidiMessageSpec::Note { note: 60 };
        assert!(note_spec.matches(&MidiMessage::NoteOn { note: 60, velocity: 100 }));
        assert!(!note_spec.matches(&MidiMessage::NoteOn { note: 61, velocity: 100 }));
        assert!(!note_spec.matches(&MidiMessage::NoteOff { note: 60 }));

        let cc_spec = MidiMessageSpec::ControlChange { cc: 34 };
        assert!(cc_spec.matches(&MidiMessage::ControlChange { cc: 34, value: 127 }));
        assert!(!cc_spec.matches(&MidiMessage::ControlChange { cc: 35, value: 127 }));
    }
}
