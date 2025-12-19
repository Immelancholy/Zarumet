use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub mpd: MpdConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpdConfig {
    #[serde(default = "MpdConfig::default_address")]
    pub address: String,
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
        }
    }
}

impl MpdConfig {
    fn default_address() -> String {
        "localhost:6600".to_string()
    }
}

impl Default for MpdConfig {
    fn default() -> Self {
        Self {
            address: Self::default_address(),
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
        "fae280".to_string()
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
