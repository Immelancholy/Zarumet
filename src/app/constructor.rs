use super::App;
use crate::app::cli::Args;
use crate::binds::KeyBinds;
use crate::config::Config;
use crate::ui::menu::{MenuMode, PanelFocus};
use ratatui::widgets::ListState;
use std::path::PathBuf;

/// Trait for App construction
#[allow(dead_code)]
pub trait AppConstructor {
    fn new(args: Args) -> color_eyre::Result<Self>
    where
        Self: Sized;
}

/// Get the path to the state file
/// - Linux: ~/.local/state/zarumet/state.toml (XDG_STATE_HOME)
/// - macOS: ~/Library/Application Support/zarumet/state.toml
/// - Windows: C:\Users\<User>\AppData\Roaming\zarumet\state.toml
fn get_state_path() -> Option<PathBuf> {
    // Use state_dir on Linux (XDG_STATE_HOME), fall back to data_dir on other platforms
    let base_dir = dirs::state_dir().or_else(dirs::data_dir)?;
    Some(base_dir.join("zarumet").join("state.toml"))
}

/// Load bit-perfect state from state file
fn load_bit_perfect_state() -> bool {
    let state_path = match get_state_path() {
        Some(path) => path,
        None => return false,
    };

    if !state_path.exists() {
        return false;
    }

    let contents = match std::fs::read_to_string(&state_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Simple parsing: look for "bit_perfect = true"
    contents
        .lines()
        .find(|line| line.starts_with("bit_perfect"))
        .and_then(|line| line.split('=').nth(1))
        .map(|val| val.trim() == "true")
        .unwrap_or(false)
}

/// Save bit-perfect state to state file
pub fn save_bit_perfect_state(enabled: bool) -> std::io::Result<()> {
    let state_path = match get_state_path() {
        Some(path) => path,
        None => return Ok(()),
    };

    // Create directory if it doesn't exist
    if let Some(parent) = state_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = format!("bit_perfect = {}\n", enabled);
    std::fs::write(&state_path, contents)
}

impl AppConstructor for App {
    /// Construct a new instance of [`App`].
    fn new(args: Args) -> color_eyre::Result<Self> {
        let config_path = args.config.clone();
        let address = args.address.clone();
        let (mut config, _warnings) = Config::load(config_path)?;

        if let Some(addr) = address {
            config.mpd.address = addr;
        }

        Self::new_with_config(config, args)
    }
}

impl App {
    /// Construct a new instance of [`App`] with pre-loaded config.
    pub fn new_with_config(config: Config, args: Args) -> color_eyre::Result<Self> {
        let queue_list_state = ListState::default();
        // Don't select anything initially - will be set when queue is populated

        // Build enhanced key maps from config
        let (global_map, queue_map, artists_map, albums_map, sequential_bindings) =
            config.binds.build_enhanced_key_maps();
        let key_binds = KeyBinds::new_with_sequential(
            global_map,
            queue_map,
            artists_map,
            albums_map,
            sequential_bindings,
        );

        // Determine bit-perfect state: CLI flag takes priority, then saved state
        let bit_perfect_enabled = match args.bit_perfect {
            Some(value) => value,             // CLI explicitly set on/off
            None => load_bit_perfect_state(), // No CLI flag, use saved state
        };

        Ok(Self {
            running: false,
            current_song: None,
            queue: Vec::new(),
            selected_queue_index: None, // Will be set when queue is populated
            queue_list_state,
            artist_list_state: ListState::default(),
            album_list_state: ListState::default(),
            album_display_list_state: ListState::default(),
            all_albums_list_state: ListState::default(),
            album_tracks_list_state: ListState::default(),
            config,
            menu_mode: MenuMode::Queue,       // Start with queue menu
            panel_focus: PanelFocus::Artists, // Start with artists panel focused
            artists_panel_focus: PanelFocus::Artists, // Default for Artists mode
            albums_panel_focus: PanelFocus::AlbumList, // Default for Albums mode
            library: None,
            expanded_albums: std::collections::HashSet::new(),
            mpd_status: None,
            key_binds,
            bit_perfect_enabled,
            force_update: true, // Force initial update
            config_warnings: Vec::new(),
            show_config_warnings_popup: false,
            last_play_state: None,
            last_sample_rate: None,
            spinner_frame: 0,
        })
    }
}
