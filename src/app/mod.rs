use crate::config::Config;
use crate::song::{Library, SongInfo};
use crate::ui::menu::{MenuMode, PanelFocus};
use ratatui::widgets::ListState;

// Module declarations
pub mod cli;
pub mod constructor;
pub mod event_handlers;
pub mod main_loop;
pub mod mpd_handler;
pub mod mpd_updates;
pub mod navigation;
pub mod terminal;

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
    /// List states for Tracks navigation
    pub artist_list_state: ListState,
    pub album_list_state: ListState,
    pub album_display_list_state: ListState, // For handling expanded album navigation
    /// Configuration loaded from TOML file
    pub config: Config,
    /// Current menu mode
    pub menu_mode: MenuMode,
    /// Current panel focus in Tracks mode
    pub panel_focus: PanelFocus,
    /// Music library
    pub library: Option<Library>,
    /// Expanded albums (tracks which albums are currently expanded)
    pub expanded_albums: std::collections::HashSet<(String, String)>, // (artist_name, album_name)
    /// Current MPD status information
    pub mpd_status: Option<mpd_client::responses::Status>,
}
