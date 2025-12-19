use crate::ui::menu::{MenuMode, PanelFocus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::app::mpd_handler::MPDAction;

/// Key binding definitions for MPD controls
#[derive(Debug)]
pub struct KeyBinds {
    global_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    queue_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    tracks_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
}

impl KeyBinds {
    pub fn new(
        global_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        queue_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        tracks_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    ) -> Self {
        Self { global_map, queue_map, tracks_map }
    }

    /// Handle key events and return corresponding MPD commands
    pub fn handle_key(
        &self,
        key: KeyEvent,
        mode: &MenuMode,
        panel_focus: &PanelFocus,
    ) -> Option<MPDAction> {
        // Check global bindings first (highest priority)
        if let Some(action) = self.global_map.get(&(key.modifiers, key.code)) {
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
                if let Some(action) = self.queue_map.get(&(key.modifiers, key.code)) {
                    return Some(action.clone());
                }
            }
            MenuMode::Tracks => {
                if let Some(action) = self.tracks_map.get(&(key.modifiers, key.code)) {
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

        // If not found in any map, return None
        None
    }
}