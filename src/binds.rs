use crate::ui::menu::{MenuMode, PanelFocus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::app::mpd_handler::MPDAction;

/// Sequential key binding configuration
#[derive(Debug, Clone)]
pub struct SequentialKeyBinding {
    pub sequence: Vec<(KeyModifiers, KeyCode)>,
    pub action: MPDAction,
}

/// Key binding state for sequential input
#[derive(Debug, Clone, PartialEq)]
pub enum KeyState {
    Idle,
    Awaiting {
        sequence: Vec<(KeyModifiers, KeyCode)>,
        timeout: Instant,
    },
}

/// Key binding definitions for MPD controls with sequential support
#[derive(Debug)]
pub struct KeyBinds {
    global_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    queue_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    tracks_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    sequential_bindings: Vec<SequentialKeyBinding>,
    current_state: KeyState,
    default_timeout: Duration,
}

impl KeyBinds {
    /// New constructor that supports sequential bindings
    pub fn new_with_sequential(
        global_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        queue_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        tracks_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        sequential_bindings: Vec<SequentialKeyBinding>,
    ) -> Self {
        Self {
            global_map,
            queue_map,
            tracks_map,
            sequential_bindings,
            current_state: KeyState::Idle,
            default_timeout: Duration::from_millis(1000),
        }
    }

    /// Handle key events and return corresponding MPD commands with sequential support
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        mode: &MenuMode,
        panel_focus: &PanelFocus,
    ) -> Option<MPDAction> {
        let key_tuple = (key.modifiers, key.code);

        // Handle sequential key state
        if !matches!(self.current_state, KeyState::Idle) {
            return self.handle_sequential_input(key_tuple, mode, panel_focus);
        }

        // Check global bindings first (highest priority)
        if let Some(action) = self.global_map.get(&key_tuple) {
            // Handle mode-specific logic for certain bindings
            match (action, mode) {
                (MPDAction::PlaySelected, MenuMode::Tracks) => {
                    // Don't allow play_selected in tracks mode - it conflicts with navigation
                    return None;
                }
                _ => return Some(action.clone()),
            }
        }

        // Check mode-specific bindings
        match mode {
            MenuMode::Queue => {
                if let Some(action) = self.queue_map.get(&key_tuple) {
                    return Some(action.clone());
                }
            }
            MenuMode::Tracks => {
                if let Some(action) = self.tracks_map.get(&key_tuple) {
                    // Handle panel-specific logic for tracks mode
                    match (action, panel_focus) {
                        (MPDAction::SwitchPanelRight, PanelFocus::Artists) => {
                            return Some(MPDAction::SwitchPanelRight);
                        }
                        (MPDAction::SwitchPanelRight, PanelFocus::Albums) => {
                            return Some(MPDAction::ToggleAlbumExpansion);
                        }
                        _ => return Some(action.clone()),
                    }
                }
            }
        }

        // Check if this key could start a sequential binding
        if self.could_start_sequence(key_tuple) {
            self.current_state = KeyState::Awaiting {
                sequence: vec![key_tuple],
                timeout: Instant::now() + self.default_timeout,
            };
            None // Waiting for more input
        } else {
            None
        }
    }

    /// Handle input when in the middle of a sequential key sequence
    fn handle_sequential_input(
        &mut self,
        key_tuple: (KeyModifiers, KeyCode),
        mode: &MenuMode,
        panel_focus: &PanelFocus,
    ) -> Option<MPDAction> {
        match &mut self.current_state {
            KeyState::Idle => None, // Should not happen
            KeyState::Awaiting { sequence, timeout } => {
                // Check timeout
                if *timeout < Instant::now() {
                    self.current_state = KeyState::Idle;
                    // Try as single key now
                    return self.handle_key(
                        KeyEvent::new(key_tuple.1, key_tuple.0),
                        mode,
                        panel_focus,
                    );
                }

                sequence.push(key_tuple);

                // Check for complete sequence match
                for binding in &self.sequential_bindings {
                    if binding.sequence == *sequence {
                        self.current_state = KeyState::Idle;
                        return Some(binding.action.clone());
                    }
                }

                // Check if current sequence could still match something
                let possible_matches: Vec<_> = self
                    .sequential_bindings
                    .iter()
                    .filter(|binding| {
                        sequence.len() <= binding.sequence.len()
                            && binding.sequence.starts_with(sequence)
                    })
                    .collect();

                if possible_matches.is_empty() {
                    // No match, reset and try first key as single key
                    let first_key = sequence[0];
                    self.current_state = KeyState::Idle;

                    // Try the first key as a single key
                    self.handle_key(KeyEvent::new(first_key.1, first_key.0), mode, panel_focus)
                } else {
                    // Continue waiting, update timeout
                    *timeout = Instant::now() + self.default_timeout;
                    None
                }
            }
        }
    }

    /// Check if a key could start a sequential binding
    fn could_start_sequence(&self, key_tuple: (KeyModifiers, KeyCode)) -> bool {
        self.sequential_bindings
            .iter()
            .any(|binding| binding.sequence.first() == Some(&key_tuple))
    }

    /// Get current key sequence for UI display
    pub fn get_current_sequence(&self) -> Vec<(KeyModifiers, KeyCode)> {
        match &self.current_state {
            KeyState::Awaiting { sequence, .. } => sequence.clone(),
            _ => Vec::new(),
        }
    }

    /// Check if currently awaiting input for a sequence
    pub fn is_awaiting_input(&self) -> bool {
        matches!(self.current_state, KeyState::Awaiting { .. })
    }

    /// Update method to handle timeouts (call this regularly)
    pub fn update(&mut self) -> Option<MPDAction> {
        if let KeyState::Awaiting { timeout, .. } = &self.current_state
            && *timeout < Instant::now()
        {
            self.current_state = KeyState::Idle;
            // No action on timeout, just reset
        }
        None
    }
}
