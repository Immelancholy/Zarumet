use super::App;
use crate::app::cli::Args;
use crate::binds::KeyBinds;
use crate::config::Config;
use crate::ui::menu::{MenuMode, PanelFocus};
use ratatui::widgets::ListState;

/// Trait for App construction
pub trait AppConstructor {
    fn new(args: Args) -> color_eyre::Result<Self>
    where
        Self: Sized;
}

impl AppConstructor for App {
    /// Construct a new instance of [`App`].
    fn new(args: Args) -> color_eyre::Result<Self> {
        let mut config = Config::load(args.config)?;

        if let Some(address) = args.address {
            config.mpd.address = address;
        }

        let queue_list_state = ListState::default();
        // Don't select anything initially - will be set when queue is populated

        // Build enhanced key maps from config
        let (global_map, queue_map, tracks_map, sequential_bindings) = config.binds.build_enhanced_key_maps();
        let key_binds = KeyBinds::new_with_sequential(global_map, queue_map, tracks_map, sequential_bindings);

        Ok(Self {
            running: false,
            current_song: None,
            queue: Vec::new(),
            selected_queue_index: None, // Will be set when queue is populated
            queue_list_state,
            artist_list_state: ListState::default(),
            album_list_state: ListState::default(),
            album_display_list_state: ListState::default(),
            config,
            menu_mode: MenuMode::Queue,       // Start with queue menu
            panel_focus: PanelFocus::Artists, // Start with artists panel focused
            library: None,
            expanded_albums: std::collections::HashSet::new(),
            mpd_status: None,
            key_binds,
        })
    }
}
