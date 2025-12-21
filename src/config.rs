use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub mpd: MpdConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
    #[serde(default)]
    pub binds: BindsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpdConfig {
    #[serde(default = "MpdConfig::default_address")]
    pub address: String,
    #[serde(default = "MpdConfig::default_volume_increment")]
    pub volume_increment: u32,
    #[serde(default = "MpdConfig::default_volume_increment_fine")]
    pub volume_increment_fine: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BindsConfig {
    #[serde(default = "BindsConfig::default_next")]
    pub next: Vec<String>,
    #[serde(default = "BindsConfig::default_previous")]
    pub previous: Vec<String>,
    #[serde(default = "BindsConfig::default_toggle_play_pause")]
    pub toggle_play_pause: Vec<String>,
    #[serde(default = "BindsConfig::default_volume_up")]
    pub volume_up: Vec<String>,
    #[serde(default = "BindsConfig::default_volume_up_fine")]
    pub volume_up_fine: Vec<String>,
    #[serde(default = "BindsConfig::default_volume_down")]
    pub volume_down: Vec<String>,
    #[serde(default = "BindsConfig::default_volume_down_fine")]
    pub volume_down_fine: Vec<String>,
    #[serde(default = "BindsConfig::default_toggle_mute")]
    pub toggle_mute: Vec<String>,
    #[serde(default = "BindsConfig::default_cycle_mode_right")]
    pub cycle_mode_right: Vec<String>,
    #[serde(default = "BindsConfig::default_cycle_mode_left")]
    pub cycle_mode_left: Vec<String>,
    #[serde(default = "BindsConfig::default_clear_queue")]
    pub clear_queue: Vec<String>,
    #[serde(default = "BindsConfig::default_repeat")]
    pub repeat: Vec<String>,
    #[serde(default = "BindsConfig::default_random")]
    pub random: Vec<String>,
    #[serde(default = "BindsConfig::default_single")]
    pub single: Vec<String>,
    #[serde(default = "BindsConfig::default_consume")]
    pub consume: Vec<String>,
    #[serde(default = "BindsConfig::default_quit")]
    pub quit: Vec<String>,
    #[serde(default = "BindsConfig::default_refresh")]
    pub refresh: Vec<String>,
    #[serde(default = "BindsConfig::default_switch_to_queue_menu")]
    pub switch_to_queue_menu: Vec<String>,
    #[serde(default = "BindsConfig::default_switch_to_tracks")]
    pub switch_to_tracks: Vec<String>,
    #[serde(default = "BindsConfig::default_seek_forward")]
    pub seek_forward: Vec<String>,
    #[serde(default = "BindsConfig::default_seek_backward")]
    pub seek_backward: Vec<String>,
    #[serde(default = "BindsConfig::default_scroll_up")]
    pub scroll_up: Vec<String>,
    #[serde(default = "BindsConfig::default_scroll_down")]
    pub scroll_down: Vec<String>,
    #[serde(default = "BindsConfig::default_play_selected")]
    pub play_selected: Vec<String>,
    #[serde(default = "BindsConfig::default_remove_from_queue")]
    pub remove_from_queue: Vec<String>,
    #[serde(default = "BindsConfig::default_move_up_in_queue")]
    pub move_up_in_queue: Vec<String>,
    #[serde(default = "BindsConfig::default_move_down_in_queue")]
    pub move_down_in_queue: Vec<String>,
    #[serde(default = "BindsConfig::default_switch_panel_left")]
    pub switch_panel_left: Vec<String>,
    #[serde(default = "BindsConfig::default_switch_panel_right")]
    pub switch_panel_right: Vec<String>,
    #[serde(default = "BindsConfig::default_toggle_album_expansion")]
    pub toggle_album_expansion: Vec<String>,
    #[serde(default = "BindsConfig::default_add_song_to_queue")]
    pub add_song_to_queue: Vec<String>,
    #[serde(default = "BindsConfig::default_scroll_up_big")]
    pub scroll_up_big: Vec<String>,
    #[serde(default = "BindsConfig::default_scroll_down_big")]
    pub scroll_down_big: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    #[serde(default = "ColorsConfig::default_border")]
    pub border: String,
    #[serde(default = "ColorsConfig::default_song_title")]
    pub song_title: String,
    #[serde(default = "ColorsConfig::default_album")]
    pub album: String,
    #[serde(default = "ColorsConfig::default_artist")]
    pub artist: String,
    #[serde(default = "ColorsConfig::default_border_title")]
    pub border_title: String,
    #[serde(default = "ColorsConfig::default_progress_filled")]
    pub progress_filled: String,
    #[serde(default = "ColorsConfig::default_progress_empty")]
    pub progress_empty: String,
    #[serde(default = "ColorsConfig::default_paused")]
    pub paused: String,
    #[serde(default = "ColorsConfig::default_playing")]
    pub playing: String,
    #[serde(default = "ColorsConfig::default_stopped")]
    pub stopped: String,
    #[serde(default = "ColorsConfig::default_time_separator")]
    pub time_separator: String,
    #[serde(default = "ColorsConfig::default_time_duration")]
    pub time_duration: String,
    #[serde(default = "ColorsConfig::default_time_elapsed")]
    pub time_elapsed: String,
    #[serde(default = "ColorsConfig::default_queue_selected_highlight")]
    pub queue_selected_highlight: String,
    #[serde(default = "ColorsConfig::default_queue_selected_text")]
    pub queue_selected_text: String,
    #[serde(default = "ColorsConfig::default_queue_album")]
    pub queue_album: String,
    #[serde(default = "ColorsConfig::default_queue_song_title")]
    pub queue_song_title: String,
    #[serde(default = "ColorsConfig::default_queue_artist")]
    pub queue_artist: String,
    #[serde(default = "ColorsConfig::default_queue_position")]
    pub queue_position: String,
    #[serde(default = "ColorsConfig::default_queue_duration")]
    pub queue_duration: String,
    #[serde(default = "ColorsConfig::default_top_accent")]
    pub top_accent: String,
    #[serde(default = "ColorsConfig::default_volume")]
    pub volume: String,
    #[serde(default = "ColorsConfig::default_volume_empty")]
    pub volume_empty: String,
    #[serde(default = "ColorsConfig::default_mode")]
    pub mode: String,
    #[serde(default = "ColorsConfig::default_track_duration")]
    pub track_duration: String,
}

impl Config {
    pub fn load(config_path: Option<PathBuf>) -> color_eyre::Result<Self> {
        let config_path = match config_path {
            Some(path) => path,
            None => {
                let home = std::env::var("HOME")?;
                PathBuf::from(home)
                    .join(".config")
                    .join("zarumet")
                    .join("config.toml")
            }
        };

        // Check if config file exists
        if !config_path.exists() {
            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Create default config
            let default_config = Config::default();

            // Serialize to TOML and write to file
            let toml_string = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, &toml_string)?;

            eprintln!("Created default config file at: {}", config_path.display());

            return Ok(default_config);
        }
        let contents = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse config file: {}", e);
            Config::default()
        });
        Ok(config)
    }
}

impl ColorsConfig {
    /// Parse a hex color string like "#FF5500" into RGB values
    pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    }

    pub fn time_elapsed(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.time_elapsed)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn time_duration(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.time_duration)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn time_separator(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.time_separator)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn paused(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.paused)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn playing(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.playing)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn stopped(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.stopped)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn album_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.album)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn progress_filled_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.progress_filled)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Green)
    }

    pub fn progress_empty_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.progress_empty)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Black)
    }

    pub fn border_title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.border_title)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn border_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.border)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn artist_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.artist)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Cyan)
    }

    pub fn song_title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.song_title)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Yellow)
    }

    pub fn queue_album_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_album)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Green)
    }

    pub fn queue_artist_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_artist)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Cyan)
    }

    pub fn queue_song_title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_song_title)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Yellow)
    }

    pub fn queue_selected_text_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_selected_text)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn queue_selected_highlight_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_selected_highlight)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Black)
    }

    pub fn queue_position_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_position)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Magenta)
    }

    pub fn queue_duration_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.queue_duration)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Magenta)
    }

    pub fn top_accent_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.top_accent)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Blue)
    }

    pub fn volume_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.volume)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Yellow)
    }

    pub fn volume_empty_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.volume_empty)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Black)
    }

    pub fn mode_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.mode)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Green)
    }

    pub fn track_duration_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.track_duration)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Red)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpd: MpdConfig::default(),
            colors: ColorsConfig::default(),
            binds: BindsConfig::default(),
        }
    }
}

impl MpdConfig {
    fn default_address() -> String {
        "localhost:6600".to_string()
    }
    fn default_volume_increment() -> u32 {
        5
    }
    fn default_volume_increment_fine() -> u32 {
        1
    }
}

impl Default for MpdConfig {
    fn default() -> Self {
        Self {
            address: Self::default_address(),
            volume_increment: Self::default_volume_increment(),
            volume_increment_fine: Self::default_volume_increment_fine(),
        }
    }
}

impl BindsConfig {
    fn default_next() -> Vec<String> {
        vec![
            ">".to_string(),
            "shift-j".to_string(),
            "shift-down".to_string(),
        ]
    }
    fn default_previous() -> Vec<String> {
        vec![
            "<".to_string(),
            "shift-k".to_string(),
            "shift-up".to_string(),
        ]
    }
    fn default_toggle_play_pause() -> Vec<String> {
        vec![" ".to_string(), "p".to_string()]
    }
    fn default_volume_up() -> Vec<String> {
        vec!["=".to_string()]
    }
    fn default_volume_up_fine() -> Vec<String> {
        vec!["+".to_string()]
    }
    fn default_volume_down() -> Vec<String> {
        vec!["-".to_string()]
    }
    fn default_volume_down_fine() -> Vec<String> {
        vec!["_".to_string()]
    }
    fn default_toggle_mute() -> Vec<String> {
        vec!["m".to_string()]
    }
    fn default_cycle_mode_right() -> Vec<String> {
        vec!["ctrl-l".to_string(), "ctrl-right".to_string()]
    }
    fn default_cycle_mode_left() -> Vec<String> {
        vec!["ctrl-h".to_string(), "ctrl-left".to_string()]
    }
    fn default_clear_queue() -> Vec<String> {
        vec!["d".to_string()]
    }
    fn default_repeat() -> Vec<String> {
        vec!["r".to_string()]
    }
    fn default_random() -> Vec<String> {
        vec!["z".to_string()]
    }
    fn default_single() -> Vec<String> {
        vec!["s".to_string()]
    }
    fn default_consume() -> Vec<String> {
        vec!["c".to_string()]
    }
    fn default_quit() -> Vec<String> {
        vec!["esc".to_string(), "q".to_string(), "ctrl-c".to_string()]
    }

    fn default_quit_enhanced() -> Vec<String> {
        vec![
            "esc".to_string(),
            "q".to_string(),
            "ctrl-c".to_string(),
            "q q".to_string(),
        ]
    }
    fn default_refresh() -> Vec<String> {
        vec!["u".to_string()]
    }
    fn default_switch_to_queue_menu() -> Vec<String> {
        vec!["1".to_string()]
    }
    fn default_switch_to_tracks() -> Vec<String> {
        vec!["2".to_string()]
    }
    fn default_seek_forward() -> Vec<String> {
        vec!["shift-l".to_string(), "shift-right".to_string()]
    }
    fn default_seek_backward() -> Vec<String> {
        vec!["shift-h".to_string(), "shift-left".to_string()]
    }
    fn default_scroll_up() -> Vec<String> {
        vec!["k".to_string(), "up".to_string()]
    }

    fn default_scroll_up_enhanced() -> Vec<String> {
        vec!["k".to_string(), "up".to_string(), "g g".to_string()]
    }
    fn default_scroll_down() -> Vec<String> {
        vec!["j".to_string(), "down".to_string()]
    }

    fn default_scroll_down_enhanced() -> Vec<String> {
        vec!["j".to_string(), "down".to_string(), "G".to_string()]
    }
    fn default_play_selected() -> Vec<String> {
        vec!["enter".to_string(), "l".to_string(), "right".to_string()]
    }
    fn default_remove_from_queue() -> Vec<String> {
        vec!["x".to_string(), "backspace".to_string()]
    }

    fn default_remove_from_queue_enhanced() -> Vec<String> {
        vec!["x".to_string(), "backspace".to_string(), "d d".to_string()]
    }
    fn default_move_up_in_queue() -> Vec<String> {
        vec!["ctrl-k".to_string(), "ctrl-up".to_string()]
    }
    fn default_move_down_in_queue() -> Vec<String> {
        vec!["ctrl-j".to_string(), "ctrl-down".to_string()]
    }
    fn default_switch_panel_left() -> Vec<String> {
        vec!["h".to_string(), "left".to_string()]
    }
    fn default_switch_panel_right() -> Vec<String> {
        vec!["l".to_string(), "right".to_string()]
    }
    fn default_toggle_album_expansion() -> Vec<String> {
        vec!["l".to_string(), "right".to_string()]
    }
    fn default_add_song_to_queue() -> Vec<String> {
        vec!["a".to_string(), "enter".to_string()]
    }
    fn default_scroll_up_big() -> Vec<String> {
        vec!["ctrl-u".to_string()]
    }
    fn default_scroll_down_big() -> Vec<String> {
        vec!["ctrl-d".to_string()]
    }

    pub fn parse_keybinding(
        &self,
        key_str: &str,
    ) -> Option<(crossterm::event::KeyModifiers, crossterm::event::KeyCode)> {
        let key_str = key_str.to_lowercase();

        // Special case for standalone "-" character
        if key_str == "-" {
            return Some((
                crossterm::event::KeyModifiers::NONE,
                crossterm::event::KeyCode::Char('-'),
            ));
        }

        let parts: Vec<&str> = key_str.split('-').collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = crossterm::event::KeyModifiers::NONE;
        let key_part = parts[parts.len() - 1];

        // Parse modifiers
        for part in &parts[..parts.len() - 1] {
            match *part {
                "ctrl" => modifiers |= crossterm::event::KeyModifiers::CONTROL,
                "alt" => modifiers |= crossterm::event::KeyModifiers::ALT,
                "shift" => modifiers |= crossterm::event::KeyModifiers::SHIFT,
                _ => return None,
            }
        }

        // Parse key code
        let code = match key_part {
            "esc" => crossterm::event::KeyCode::Esc,
            "enter" => crossterm::event::KeyCode::Enter,
            "backspace" => crossterm::event::KeyCode::Backspace,
            "tab" => crossterm::event::KeyCode::Tab,
            "delete" => crossterm::event::KeyCode::Delete,
            "insert" => crossterm::event::KeyCode::Insert,
            "home" => crossterm::event::KeyCode::Home,
            "end" => crossterm::event::KeyCode::End,
            "pageup" => crossterm::event::KeyCode::PageUp,
            "pagedown" => crossterm::event::KeyCode::PageDown,
            "up" => crossterm::event::KeyCode::Up,
            "down" => crossterm::event::KeyCode::Down,
            "left" => crossterm::event::KeyCode::Left,
            "right" => crossterm::event::KeyCode::Right,
            "f1" => crossterm::event::KeyCode::F(1),
            "f2" => crossterm::event::KeyCode::F(2),
            "f3" => crossterm::event::KeyCode::F(3),
            "f4" => crossterm::event::KeyCode::F(4),
            "f5" => crossterm::event::KeyCode::F(5),
            "f6" => crossterm::event::KeyCode::F(6),
            "f7" => crossterm::event::KeyCode::F(7),
            "f8" => crossterm::event::KeyCode::F(8),
            "f9" => crossterm::event::KeyCode::F(9),
            "f10" => crossterm::event::KeyCode::F(10),
            "f11" => crossterm::event::KeyCode::F(11),
            "f12" => crossterm::event::KeyCode::F(12),
            // Handle special single-character keys
            "space" => crossterm::event::KeyCode::Char(' '),
            // Handle characters - if shift is present, capitalize
            c if c.len() == 1 => {
                let char_bytes = c.chars().next().unwrap();
                if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    crossterm::event::KeyCode::Char(char_bytes.to_ascii_uppercase())
                } else {
                    crossterm::event::KeyCode::Char(char_bytes)
                }
            }
            _ => return None,
        };

        Some((modifiers, code))
    }

    /// Parse a binding string that may contain space-separated sequential keys
    /// Returns a vector of parsed key tuples
    pub fn parse_binding_string(
        &self,
        binding_str: &str,
    ) -> Vec<(crossterm::event::KeyModifiers, crossterm::event::KeyCode)> {
        binding_str
            .split_whitespace()
            .filter_map(|key_str| self.parse_keybinding(key_str))
            .collect()
    }

    /// Build enhanced key maps with sequential key support
    pub fn build_enhanced_key_maps(
        &self,
    ) -> (
        HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        Vec<crate::binds::SequentialKeyBinding>,
    ) {
        self.build_enhanced_key_maps_internal()
    }

    /// Internal implementation for building enhanced key maps
    fn build_enhanced_key_maps_internal(
        &self,
    ) -> (
        HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        Vec<crate::binds::SequentialKeyBinding>,
    ) {
        let mut global_map = HashMap::new();
        let mut queue_map = HashMap::new();
        let mut tracks_map = HashMap::new();
        let mut sequential_bindings = Vec::new();

        // Global bindings (always available)
        self.add_enhanced_global_bindings(&mut global_map, &mut sequential_bindings);

        // Queue mode specific bindings
        self.add_enhanced_queue_bindings(&mut queue_map, &mut sequential_bindings);

        // Tracks mode specific bindings
        self.add_enhanced_tracks_bindings(&mut tracks_map, &mut sequential_bindings);

        (global_map, queue_map, tracks_map, sequential_bindings)
    }

    fn add_enhanced_global_bindings(
        &self,
        single_map: &mut HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        sequential_bindings: &mut Vec<crate::binds::SequentialKeyBinding>,
    ) {
        // Global bindings - these work in all modes
        self.add_enhanced_binding_for_action(
            &self.next,
            crate::app::mpd_handler::MPDAction::Next,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.previous,
            crate::app::mpd_handler::MPDAction::Previous,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.toggle_play_pause,
            crate::app::mpd_handler::MPDAction::TogglePlayPause,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.volume_up,
            crate::app::mpd_handler::MPDAction::VolumeUp,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.volume_up_fine,
            crate::app::mpd_handler::MPDAction::VolumeUpFine,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.volume_down,
            crate::app::mpd_handler::MPDAction::VolumeDown,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.volume_down_fine,
            crate::app::mpd_handler::MPDAction::VolumeDownFine,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.toggle_mute,
            crate::app::mpd_handler::MPDAction::ToggleMute,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.cycle_mode_right,
            crate::app::mpd_handler::MPDAction::CycleModeRight,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.cycle_mode_left,
            crate::app::mpd_handler::MPDAction::CycleModeLeft,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.clear_queue,
            crate::app::mpd_handler::MPDAction::ClearQueue,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.repeat,
            crate::app::mpd_handler::MPDAction::Repeat,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.random,
            crate::app::mpd_handler::MPDAction::Random,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.single,
            crate::app::mpd_handler::MPDAction::Single,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.consume,
            crate::app::mpd_handler::MPDAction::Consume,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.quit,
            crate::app::mpd_handler::MPDAction::Quit,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.refresh,
            crate::app::mpd_handler::MPDAction::Refresh,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.switch_to_queue_menu,
            crate::app::mpd_handler::MPDAction::SwitchToQueueMenu,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.switch_to_tracks,
            crate::app::mpd_handler::MPDAction::SwitchToTracks,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.seek_forward,
            crate::app::mpd_handler::MPDAction::SeekForward,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.seek_backward,
            crate::app::mpd_handler::MPDAction::SeekBackward,
            single_map,
            sequential_bindings,
        );
    }

    /// Helper method to add bindings that may be sequential
    fn add_enhanced_binding_for_action(
        &self,
        binding_strings: &[String],
        action: crate::app::mpd_handler::MPDAction,
        single_map: &mut HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        sequential_bindings: &mut Vec<crate::binds::SequentialKeyBinding>,
    ) {
        for binding_str in binding_strings {
            let key_sequence = self.parse_binding_string(binding_str);

            if key_sequence.len() == 1 {
                // Single key binding
                single_map.insert(key_sequence[0], action.clone());
            } else if key_sequence.len() > 1 {
                // Sequential key binding
                sequential_bindings.push(crate::binds::SequentialKeyBinding {
                    sequence: key_sequence,
                    action: action.clone(),
                });
            }
        }
    }

    fn add_enhanced_queue_bindings(
        &self,
        single_map: &mut HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        sequential_bindings: &mut Vec<crate::binds::SequentialKeyBinding>,
    ) {
        // Queue mode specific bindings
        self.add_enhanced_binding_for_action(
            &self.scroll_up,
            crate::app::mpd_handler::MPDAction::QueueUp,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_down,
            crate::app::mpd_handler::MPDAction::QueueDown,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.play_selected,
            crate::app::mpd_handler::MPDAction::PlaySelected,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.remove_from_queue,
            crate::app::mpd_handler::MPDAction::RemoveFromQueue,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.move_up_in_queue,
            crate::app::mpd_handler::MPDAction::MoveUpInQueue,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.move_down_in_queue,
            crate::app::mpd_handler::MPDAction::MoveDownInQueue,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_up_big,
            crate::app::mpd_handler::MPDAction::ScrollUp,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_down_big,
            crate::app::mpd_handler::MPDAction::ScrollDown,
            single_map,
            sequential_bindings,
        );
    }

    fn add_enhanced_tracks_bindings(
        &self,
        single_map: &mut HashMap<
            (crossterm::event::KeyModifiers, crossterm::event::KeyCode),
            crate::app::mpd_handler::MPDAction,
        >,
        sequential_bindings: &mut Vec<crate::binds::SequentialKeyBinding>,
    ) {
        // Tracks mode specific bindings
        self.add_enhanced_binding_for_action(
            &self.switch_panel_left,
            crate::app::mpd_handler::MPDAction::SwitchPanelLeft,
            single_map,
            sequential_bindings,
        );

        // Note: toggle_album_expansion is added first so switch_panel_right can overwrite it
        // This allows us to use the same keys for both actions with different behavior
        self.add_enhanced_binding_for_action(
            &self.toggle_album_expansion,
            crate::app::mpd_handler::MPDAction::ToggleAlbumExpansion,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.switch_panel_right,
            crate::app::mpd_handler::MPDAction::SwitchPanelRight,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_up,
            crate::app::mpd_handler::MPDAction::NavigateUp,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_down,
            crate::app::mpd_handler::MPDAction::NavigateDown,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.add_song_to_queue,
            crate::app::mpd_handler::MPDAction::AddSongToQueue,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_up_big,
            crate::app::mpd_handler::MPDAction::ScrollUp,
            single_map,
            sequential_bindings,
        );
        self.add_enhanced_binding_for_action(
            &self.scroll_down_big,
            crate::app::mpd_handler::MPDAction::ScrollDown,
            single_map,
            sequential_bindings,
        );
    }
}

impl Default for BindsConfig {
    fn default() -> Self {
        Self {
            next: Self::default_next(),
            previous: Self::default_previous(),
            toggle_play_pause: Self::default_toggle_play_pause(),
            volume_up: Self::default_volume_up(),
            volume_up_fine: Self::default_volume_up_fine(),
            volume_down: Self::default_volume_down(),
            volume_down_fine: Self::default_volume_down_fine(),
            toggle_mute: Self::default_toggle_mute(),
            cycle_mode_right: Self::default_cycle_mode_right(),
            cycle_mode_left: Self::default_cycle_mode_left(),
            clear_queue: Self::default_clear_queue(),
            repeat: Self::default_repeat(),
            random: Self::default_random(),
            single: Self::default_single(),
            consume: Self::default_consume(),
            quit: Self::default_quit_enhanced(),
            refresh: Self::default_refresh(),
            switch_to_queue_menu: Self::default_switch_to_queue_menu(),
            switch_to_tracks: Self::default_switch_to_tracks(),
            seek_forward: Self::default_seek_forward(),
            seek_backward: Self::default_seek_backward(),
            play_selected: Self::default_play_selected(),
            remove_from_queue: Self::default_remove_from_queue_enhanced(),
            move_up_in_queue: Self::default_move_up_in_queue(),
            move_down_in_queue: Self::default_move_down_in_queue(),
            switch_panel_left: Self::default_switch_panel_left(),
            switch_panel_right: Self::default_switch_panel_right(),
            toggle_album_expansion: Self::default_toggle_album_expansion(),
            add_song_to_queue: Self::default_add_song_to_queue(),
            scroll_up_big: Self::default_scroll_up_big(),
            scroll_down_big: Self::default_scroll_down_big(),
            scroll_up: Self::default_scroll_up_enhanced(),
            scroll_down: Self::default_scroll_down_enhanced(),
        }
    }
}

impl ColorsConfig {
    fn default_border() -> String {
        "#fae280".to_string()
    }

    fn default_song_title() -> String {
        "#fae280".to_string()
    }

    fn default_album() -> String {
        "#26a0a1".to_string()
    }

    fn default_artist() -> String {
        "#d67751".to_string()
    }

    fn default_border_title() -> String {
        "#8193af".to_string()
    }

    fn default_progress_filled() -> String {
        "#26a0a1".to_string()
    }

    fn default_progress_empty() -> String {
        "#1b1d0e".to_string()
    }

    fn default_paused() -> String {
        "#e16a7c".to_string()
    }

    fn default_playing() -> String {
        "#e16a7c".to_string()
    }

    fn default_stopped() -> String {
        "#e16a7c".to_string()
    }

    fn default_time_separator() -> String {
        "#e16a7c".to_string()
    }

    fn default_time_duration() -> String {
        "#e16a7c".to_string()
    }

    fn default_time_elapsed() -> String {
        "#e16a7c".to_string()
    }

    fn default_queue_selected_highlight() -> String {
        "#b18a4a".to_string()
    }

    fn default_queue_selected_text() -> String {
        "#1b1d0e".to_string()
    }

    fn default_queue_album() -> String {
        "#26a0a1".to_string()
    }

    fn default_queue_artist() -> String {
        "#d67751".to_string()
    }

    fn default_queue_song_title() -> String {
        "#fae280".to_string()
    }

    fn default_queue_position() -> String {
        "#e16a7c".to_string()
    }

    fn default_queue_duration() -> String {
        "#e16a7c".to_string()
    }

    fn default_volume() -> String {
        "#26a0a1".to_string()
    }

    fn default_top_accent() -> String {
        "#e16a7c".to_string()
    }

    fn default_track_duration() -> String {
        "#e16a7c".to_string()
    }

    fn default_volume_empty() -> String {
        "#1b1d0e".to_string()
    }

    fn default_mode() -> String {
        "#fae280".to_string()
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            album: Self::default_album(),
            artist: Self::default_artist(),
            song_title: Self::default_song_title(),
            border: Self::default_border(),
            border_title: Self::default_border_title(),
            playing: Self::default_playing(),
            paused: Self::default_paused(),
            stopped: Self::default_stopped(),
            progress_filled: Self::default_progress_filled(),
            progress_empty: Self::default_progress_empty(),
            time_elapsed: Self::default_time_elapsed(),
            time_separator: Self::default_time_separator(),
            time_duration: Self::default_time_duration(),
            queue_selected_highlight: Self::default_queue_selected_highlight(),
            queue_selected_text: Self::default_queue_selected_text(),
            queue_album: Self::default_queue_album(),
            queue_artist: Self::default_queue_artist(),
            queue_song_title: Self::default_queue_song_title(),
            queue_position: Self::default_queue_position(),
            queue_duration: Self::default_queue_duration(),
            top_accent: Self::default_top_accent(),
            volume: Self::default_volume(),
            volume_empty: Self::default_volume_empty(),
            mode: Self::default_mode(),
            track_duration: Self::default_track_duration(),
        }
    }
}
