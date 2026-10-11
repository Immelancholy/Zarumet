use crate::app::{MenuMode, PanelFocus};
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
    artists_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    albums_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    directories_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    directory_sequences: Vec<SequentialKeyBinding>,
    playlists_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    playlist_sequences: Vec<SequentialKeyBinding>,
    sequential_bindings: Vec<SequentialKeyBinding>,
    current_state: KeyState,
    default_timeout: Duration,
}

impl KeyBinds {
    /// New constructor that supports sequential bindings
    pub fn new_with_sequential(
        global_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        queue_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        artists_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        albums_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
        sequential_bindings: Vec<SequentialKeyBinding>,
    ) -> Self {
        Self {
            global_map,
            queue_map,
            artists_map,
            albums_map,
            directories_map: HashMap::new(),
            directory_sequences: Vec::new(),
            playlists_map: HashMap::new(),
            playlist_sequences: Vec::new(),
            sequential_bindings,
            current_state: KeyState::Idle,
            default_timeout: Duration::from_millis(1000),
        }
    }

    /// Keep playlist actions scoped without changing existing modes' maps.
    pub fn configure_playlists(&mut self, config: &crate::app::config::binds::BindsConfig) {
        for (bindings, action) in [
            (&config.scroll_up, MPDAction::NavigateUp),
            (&config.scroll_down, MPDAction::NavigateDown),
            (&config.switch_panel_left, MPDAction::SwitchPanelLeft),
            (&config.switch_panel_right, MPDAction::SwitchPanelRight),
            (&config.scroll_up_big, MPDAction::ScrollUp),
            (&config.scroll_down_big, MPDAction::ScrollDown),
            (&config.go_to_top, MPDAction::GoToTop),
            (&config.go_to_bottom, MPDAction::GoToBottom),
            (&config.play_playlist, MPDAction::PlayPlaylist),
            (&config.rename_playlist, MPDAction::RenamePlaylist),
            (&config.delete_playlist, MPDAction::DeletePlaylist),
            (&config.add_playlist_to_queue, MPDAction::AddPlaylistToQueue),
            (
                &config.save_queue_as_playlist,
                MPDAction::SaveQueueAsPlaylist,
            ),
        ] {
            config.add_enhanced_binding_for_action(
                bindings,
                action,
                &mut self.playlists_map,
                &mut self.playlist_sequences,
            );
        }
    }

    /// Keep directory actions scoped without changing existing modes' maps.
    pub fn configure_directories(&mut self, config: &crate::app::config::binds::BindsConfig) {
        for (bindings, action) in [
            (&config.switch_panel_left, MPDAction::SwitchPanelLeft),
            (&config.switch_panel_right, MPDAction::SwitchPanelRight),
            (&config.scroll_up, MPDAction::NavigateUp),
            (&config.scroll_down, MPDAction::NavigateDown),
            (&config.scroll_up_big, MPDAction::ScrollUp),
            (&config.scroll_down_big, MPDAction::ScrollDown),
            (&config.go_to_top, MPDAction::GoToTop),
            (&config.go_to_bottom, MPDAction::GoToBottom),
            (&config.open_directory, MPDAction::OpenDirectory),
            (&config.parent_directory, MPDAction::ParentDirectory),
            (
                &config.add_directory_selection,
                MPDAction::AddDirectorySelection,
            ),
        ] {
            config.add_enhanced_binding_for_action(
                bindings,
                action,
                &mut self.directories_map,
                &mut self.directory_sequences,
            );
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
                (
                    MPDAction::PlaySelected,
                    MenuMode::Artists | MenuMode::Years | MenuMode::Genres,
                ) => {
                    // Don't allow play_selected in Artist, Years and Genres mode - it conflicts with navigation
                    return None;
                }
                _ => return Some(action.clone()),
            }
        }

        // Check mode-specific bindings
        match mode {
            MenuMode::Directories => {
                if let Some(action) = self.directories_map.get(&key_tuple) {
                    return Some(action.clone());
                }
            }
            MenuMode::Playlists => {
                if let Some(action) = self.playlists_map.get(&key_tuple) {
                    return Some(action.clone());
                }
            }
            MenuMode::Queue => {
                if let Some(action) = self.queue_map.get(&key_tuple) {
                    return Some(action.clone());
                }
            }
            MenuMode::Artists => {
                if let Some(action) = self.artists_map.get(&key_tuple) {
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
            MenuMode::Albums => {
                // Check albums-specific bindings first
                if let Some(action) = self.albums_map.get(&key_tuple) {
                    // Handle panel-specific logic for albums mode
                    match (action, panel_focus) {
                        // PlaySelected: In AlbumTracks panel adds song, in AlbumList switches panel
                        (MPDAction::PlaySelected, PanelFocus::AlbumTracks) => {
                            return Some(MPDAction::PlaySelected);
                        }
                        (MPDAction::PlaySelected, PanelFocus::AlbumList) => {
                            return Some(MPDAction::SwitchPanelRight);
                        }
                        // AddSongToQueue: In AlbumTracks adds song, in AlbumList adds album
                        (MPDAction::AddSongToQueue, _) => {
                            return Some(MPDAction::AddSongToQueue);
                        }
                        _ => return Some(action.clone()),
                    }
                }
                // Fall back to artists_map for navigation bindings
                if let Some(action) = self.artists_map.get(&key_tuple) {
                    match (action, panel_focus) {
                        (MPDAction::SwitchPanelRight, PanelFocus::AlbumList) => {
                            return Some(MPDAction::SwitchPanelRight);
                        }
                        (MPDAction::SwitchPanelRight, PanelFocus::AlbumTracks) => {
                            // Already at rightmost panel, no action
                            return None;
                        }
                        // Skip album expansion actions in albums mode
                        (MPDAction::ToggleAlbumExpansion, _) => {
                            return None;
                        }
                        _ => return Some(action.clone()),
                    }
                }
            }
            MenuMode::Years => {
                if let Some(action) = self.artists_map.get(&key_tuple) {
                    // Handle panel-specific logic for years mode
                    match (action, panel_focus) {
                        (MPDAction::SwitchPanelRight, PanelFocus::YearList) => {
                            return Some(MPDAction::SwitchPanelRight);
                        }
                        (MPDAction::SwitchPanelRight, PanelFocus::YearAlbums) => {
                            return Some(MPDAction::ToggleAlbumExpansion);
                        }
                        _ => return Some(action.clone()),
                    }
                }
            }
            MenuMode::Genres => {
                if let Some(action) = self.artists_map.get(&key_tuple) {
                    // Handle panel-specific logic for genres mode
                    match (action, panel_focus) {
                        (MPDAction::SwitchPanelRight, PanelFocus::GenreList) => {
                            return Some(MPDAction::SwitchPanelRight);
                        }
                        (MPDAction::SwitchPanelRight, PanelFocus::GenreAlbums) => {
                            return Some(MPDAction::ToggleAlbumExpansion);
                        }
                        _ => return Some(action.clone()),
                    }
                }
            }
        }

        // Check if this key could start a sequential binding
        if self.could_start_sequence(key_tuple, mode) {
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
                for binding in self.sequential_bindings.iter().chain(
                    self.playlist_sequences
                        .iter()
                        .filter(|_| *mode == MenuMode::Playlists)
                        .chain(
                            self.directory_sequences
                                .iter()
                                .filter(|_| *mode == MenuMode::Directories),
                        ),
                ) {
                    if binding.sequence == *sequence {
                        self.current_state = KeyState::Idle;
                        return Some(binding.action.clone());
                    }
                }

                // Check if current sequence could still match something
                let possible_matches: Vec<_> = self
                    .sequential_bindings
                    .iter()
                    .chain(
                        self.playlist_sequences
                            .iter()
                            .filter(|_| *mode == MenuMode::Playlists)
                            .chain(
                                self.directory_sequences
                                    .iter()
                                    .filter(|_| *mode == MenuMode::Directories),
                            ),
                    )
                    .filter(|binding| {
                        sequence.len() <= binding.sequence.len()
                            && binding.sequence.starts_with(sequence)
                    })
                    .collect();

                if possible_matches.is_empty() {
                    // No match, reset sequence without fallback
                    self.current_state = KeyState::Idle;
                    None
                } else {
                    // Continue waiting, update timeout
                    *timeout = Instant::now() + self.default_timeout;
                    None
                }
            }
        }
    }

    /// Check if a key could start a sequential binding
    fn could_start_sequence(&self, key_tuple: (KeyModifiers, KeyCode), mode: &MenuMode) -> bool {
        self.sequential_bindings
            .iter()
            .chain(
                self.playlist_sequences
                    .iter()
                    .filter(|_| *mode == MenuMode::Playlists)
                    .chain(
                        self.directory_sequences
                            .iter()
                            .filter(|_| *mode == MenuMode::Directories),
                    ),
            )
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

#[cfg(test)]
mod tests {
    use crate::app::binds_handler::KeyBinds;
    use crate::app::config::binds::BindsConfig;
    use crate::app::mpd_handler::MPDAction;
    use crate::app::ui::{MenuMode, PanelFocus};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn keys(config: BindsConfig) -> KeyBinds {
        let (global, queue, artists, albums, sequences) = config.build_enhanced_key_maps();
        let mut binds = KeyBinds::new_with_sequential(global, queue, artists, albums, sequences);
        binds.configure_playlists(&config);
        binds.configure_directories(&config);
        binds
    }
    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn existing_mode_numbers_keep_their_meaning() {
        let mut binds = keys(BindsConfig::default());
        for (c, expected) in [
            ('1', "SwitchToQueueMenu"),
            ('2', "SwitchToArtists"),
            ('3', "SwitchToAlbums"),
            ('4', "SwitchToYears"),
            ('5', "SwitchToGenres"),
            ('6', "SwitchToDirectories"),
            ('7', "SwitchToPlaylists"),
        ] {
            let action = binds
                .handle_key(key(c), &MenuMode::Queue, &PanelFocus::Artists)
                .unwrap();
            assert_eq!(action.to_string(), expected);
        }
    }

    #[test]
    fn partial_destructive_sequence_does_nothing() {
        let mut binds = keys(BindsConfig::default());
        assert!(
            binds
                .handle_key(key('d'), &MenuMode::Queue, &PanelFocus::Artists)
                .is_none()
        );
        assert!(binds.is_awaiting_input());
        assert!(matches!(
            binds.handle_key(key('d'), &MenuMode::Queue, &PanelFocus::Artists),
            Some(MPDAction::ClearQueue)
        ));
        assert!(!binds.is_awaiting_input());
    }

    #[test]
    fn empty_configuration_binding_disables_clear() {
        let config: BindsConfig = toml::from_str("clear_queue = []").unwrap();
        let mut binds = keys(config);
        for _ in 0..2 {
            assert!(
                binds
                    .handle_key(key('d'), &MenuMode::Queue, &PanelFocus::Artists)
                    .is_none()
            );
        }
        assert!(!binds.is_awaiting_input());
    }

    #[test]
    fn global_binding_precedes_mode_binding() {
        let config = BindsConfig {
            quit: vec!["x".into()],
            ..Default::default()
        };
        let mut binds = keys(config);
        assert!(matches!(
            binds.handle_key(key('x'), &MenuMode::Queue, &PanelFocus::Artists),
            Some(MPDAction::Quit)
        ));
    }

    #[test]
    fn navigation_enters_album_without_playing() {
        let mut binds = keys(BindsConfig::default());
        assert!(matches!(
            binds.handle_key(key('l'), &MenuMode::Artists, &PanelFocus::Artists),
            Some(MPDAction::SwitchPanelRight)
        ));
        assert!(matches!(
            binds.handle_key(key('l'), &MenuMode::Artists, &PanelFocus::Albums),
            Some(MPDAction::ToggleAlbumExpansion)
        ));
    }

    #[test]
    fn directory_bindings_are_scoped_and_global_bindings_keep_priority() {
        let config = BindsConfig::default();
        let mut binds = keys(config);
        assert!(matches!(
            binds.handle_key(key('a'), &MenuMode::Directories, &PanelFocus::Artists),
            Some(MPDAction::AddDirectorySelection)
        ));
        assert!(matches!(
            binds.handle_key(key('l'), &MenuMode::Directories, &PanelFocus::Artists),
            Some(MPDAction::SwitchPanelRight)
        ));
        assert!(!matches!(
            binds.handle_key(key('a'), &MenuMode::Queue, &PanelFocus::Artists),
            Some(MPDAction::AddDirectorySelection)
        ));
        assert!(matches!(
            binds.handle_key(key('l'), &MenuMode::Artists, &PanelFocus::Artists),
            Some(MPDAction::SwitchPanelRight)
        ));
        let config = BindsConfig {
            quit: vec!["a".into()],
            ..Default::default()
        };
        let mut binds = keys(config);
        assert!(matches!(
            binds.handle_key(key('a'), &MenuMode::Directories, &PanelFocus::Artists),
            Some(MPDAction::Quit)
        ));
    }
}
