use crate::ui::menu::{MenuMode, PanelFocus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::mpd_handler::MPDAction;

/// Key binding definitions for MPD controls
pub struct KeyBinds;

impl KeyBinds {
    /// Handle key events and return corresponding MPD commands
    pub fn handle_key(
        key: KeyEvent,
        mode: &MenuMode,
        panel_focus: &PanelFocus,
    ) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Global keybindings (work in all modes)
            (KeyModifiers::NONE, KeyCode::Char(' ')) => Some(MPDAction::TogglePlayPause),
            (KeyModifiers::NONE, KeyCode::Char('p')) => Some(MPDAction::TogglePlayPause),
            (KeyModifiers::NONE, KeyCode::Char('>'))
            | (KeyModifiers::SHIFT, KeyCode::Char('J'))
            | (KeyModifiers::SHIFT, KeyCode::Down) => Some(MPDAction::Next),
            (KeyModifiers::NONE, KeyCode::Char('<'))
            | (KeyModifiers::SHIFT, KeyCode::Char('K'))
            | (KeyModifiers::SHIFT, KeyCode::Up) => Some(MPDAction::Previous),
            (KeyModifiers::NONE, KeyCode::Char('=')) | (KeyModifiers::NONE, KeyCode::Char('+')) => {
                Some(MPDAction::VolumeUp)
            }
            (KeyModifiers::NONE, KeyCode::Char('-')) | (KeyModifiers::NONE, KeyCode::Char('_')) => {
                Some(MPDAction::VolumeDown)
            }
            (KeyModifiers::NONE, KeyCode::Char('m')) => Some(MPDAction::ToggleMute),
            (KeyModifiers::CONTROL, KeyCode::Char('l'))
            | (KeyModifiers::CONTROL, KeyCode::Right) => Some(MPDAction::CycleModeRight),
            (KeyModifiers::CONTROL, KeyCode::Char('h'))
            | (KeyModifiers::CONTROL, KeyCode::Left) => Some(MPDAction::CycleModeLeft),
            (KeyModifiers::NONE, KeyCode::Char('d')) => Some(MPDAction::ClearQueue),
            (KeyModifiers::NONE, KeyCode::Char('r')) => Some(MPDAction::Repeat),
            (KeyModifiers::NONE, KeyCode::Char('z')) => Some(MPDAction::Random),
            (KeyModifiers::NONE, KeyCode::Char('s')) => Some(MPDAction::Single),
            (KeyModifiers::NONE, KeyCode::Char('c')) => Some(MPDAction::Consume),
            (KeyModifiers::NONE, KeyCode::Esc)
            | (KeyModifiers::NONE, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(MPDAction::Quit),
            (KeyModifiers::NONE, KeyCode::Char('u')) => Some(MPDAction::Refresh),
            (KeyModifiers::NONE, KeyCode::Char('1')) => Some(MPDAction::SwitchToQueueMenu),
            (KeyModifiers::NONE, KeyCode::Char('2')) => Some(MPDAction::SwitchToTracks),
            // Alternative seek bindings since Ctrl+H/L are used for mode cycling
            (KeyModifiers::SHIFT, KeyCode::Char('L')) | (KeyModifiers::SHIFT, KeyCode::Right) => {
                Some(MPDAction::SeekForward)
            }
            (KeyModifiers::SHIFT, KeyCode::Char('H')) | (KeyModifiers::SHIFT, KeyCode::Left) => {
                Some(MPDAction::SeekBackward)
            }

            // Mode-specific keybindings
            _ => match mode {
                MenuMode::Queue => Self::handle_queue_mode_key(key),
                MenuMode::Tracks => Self::handle_tracks_mode_key(key, panel_focus),
            },
        }
    }

    /// Handle keys specific to Queue mode
    fn handle_queue_mode_key(key: KeyEvent) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Queue navigation
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                Some(MPDAction::QueueDown)
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                Some(MPDAction::QueueUp)
            }
            (KeyModifiers::NONE, KeyCode::Enter)
            | (KeyModifiers::NONE, KeyCode::Char('l'))
            | (KeyModifiers::NONE, KeyCode::Right) => Some(MPDAction::PlaySelected),

            // Queue management
            (KeyModifiers::NONE, KeyCode::Char('x')) | (KeyModifiers::NONE, KeyCode::Backspace) => {
                Some(MPDAction::RemoveFromQueue)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('k')) | (KeyModifiers::CONTROL, KeyCode::Up) => {
                Some(MPDAction::MoveUpInQueue)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('j'))
            | (KeyModifiers::CONTROL, KeyCode::Down) => Some(MPDAction::MoveDownInQueue),

            _ => None,
        }
    }

    /// Handle keys specific to Tracks mode
    fn handle_tracks_mode_key(key: KeyEvent, panel_focus: &PanelFocus) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Panel switching
            (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
                Some(MPDAction::SwitchPanelLeft)
            }

            // Right navigation - different behavior based on panel focus
            (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
                match panel_focus {
                    PanelFocus::Artists => Some(MPDAction::SwitchPanelRight),
                    PanelFocus::Albums => Some(MPDAction::ToggleAlbumExpansion),
                }
            }

            // Navigation (up/down)
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                Some(MPDAction::NavigateDown)
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                Some(MPDAction::NavigateUp)
            }

            // Action keys
            (KeyModifiers::NONE, KeyCode::Char('a')) | (KeyModifiers::NONE, KeyCode::Enter) => {
                Some(MPDAction::AddSongToQueue)
            }

            _ => None,
        }
    }
}


