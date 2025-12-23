use crate::binds::KeyBinds;
use crate::config::Config;
use crate::song::{LazyLibrary, SongInfo};
use crate::ui::DirtyFlags;
use crate::ui::menu::{MenuMode, PanelFocus};
use mpd_client::responses::PlayState;
use ratatui::widgets::ListState;

// Module declarations
pub mod cli;
pub mod constructor;
pub mod cover_cache;
pub mod event_handlers;
pub mod main_loop;
pub mod mpd_handler;
pub mod mpd_updates;
pub mod navigation;
pub mod terminal;

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub created_at: std::time::Instant,
    pub message_type: MessageType,
}

#[derive(Debug, Clone)]
pub enum MessageType {
    InProgress,
    Success,
    Error,
}

/// The main application which holds the state and logic of the application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Current song information
    pub current_song: Option<SongInfo>,
    /// MPD queue information
    pub queue: Vec<SongInfo>,
    /// Currently selected queue item index
    pub selected_queue_index: Option<usize>,
    /// List state for the queue widget
    pub queue_list_state: ListState,
    /// List states for Artists navigation
    pub artist_list_state: ListState,
    pub album_list_state: ListState,
    pub album_display_list_state: ListState, // For handling expanded album navigation
    /// List states for Albums mode navigation (separate from Artists mode)
    pub all_albums_list_state: ListState, // For navigating all_albums in Albums mode
    pub album_tracks_list_state: ListState,  // For navigating tracks within an album in Albums mode
    /// Configuration loaded from TOML file
    pub config: Config,
    /// Current menu mode
    pub menu_mode: MenuMode,
    /// Current panel focus in Artists mode
    pub panel_focus: PanelFocus,
    /// Cached panel focus for Artists mode (restored when switching back)
    pub artists_panel_focus: PanelFocus,
    /// Cached panel focus for Albums mode (restored when switching back)
    pub albums_panel_focus: PanelFocus,
    /// Music library (lazy-loaded)
    pub library: Option<LazyLibrary>,
    /// Expanded albums (tracks which albums are currently expanded)
    pub expanded_albums: std::collections::HashSet<(String, String)>, // (artist_name, album_name)
    /// Current MPD status information
    pub mpd_status: Option<mpd_client::responses::Status>,
    /// Key bindings handler
    pub key_binds: KeyBinds,
    /// Bit-perfect mode enabled (PipeWire sample rate matching)
    pub bit_perfect_enabled: bool,
    /// Flag to force immediate MPD status update (set after user actions)
    pub force_update: bool,
    /// Config warnings to display in popup
    pub config_warnings: Vec<String>,
    /// Whether the config warnings popup is currently showing
    pub show_config_warnings_popup: bool,
    /// Last play state for PipeWire rate tracking (used to detect state changes)
    pub last_play_state: Option<PlayState>,
    /// Last sample rate for PipeWire rate tracking (used to detect song changes)
    pub last_sample_rate: Option<u32>,
    /// Last known queue/playlist version from MPD (for differential updates)
    pub last_playlist_version: Option<u32>,
    /// Last known song ID from MPD (to skip refetching same song)
    pub last_song_id: Option<mpd_client::commands::SongId>,
    /// Dirty flags for optimized rendering (tracks which UI regions need redraw)
    pub dirty: DirtyFlags,
    /// Flag to indicate library reload is needed
    pub library_reload_pending: bool,
    /// Previous artist index to restore after library reload
    pub pending_artist_index: Option<String>,
    /// Library reload status message
    pub status_message: Option<StatusMessage>,
    /// Track if update is in progress to avoid overlapping updates
    pub update_in_progress: bool,
}

impl App {
    pub fn set_status_message(&mut self, message: StatusMessage) {
        self.status_message = Some(message);
        self.dirty.mark_status_message();
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
        self.dirty.mark_status_message();
    }

    pub fn check_status_message_expiry(&mut self) {
        if let Some(msg) = &self.status_message {
            let duration = match msg.message_type {
                MessageType::InProgress => std::time::Duration::from_secs(300), // Longer for in-progress
                _ => std::time::Duration::from_secs(5), // Shorter for success/error
            };
            if msg.created_at.elapsed() >= duration {
                self.clear_status_message();
            }
        }
    }
}
