use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub mpd: MpdConfig,
    pub colors: ColorsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpdConfig {
    pub address: String,
    pub music_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ColorsConfig {
    pub border: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub status: String,
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
        let config: Config = toml::from_str(&contents)?;
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

    pub fn album_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.album)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::White)
    }

    pub fn status_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.status)
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

    pub fn title_color(&self) -> ratatui::style::Color {
        Self::parse_hex(&self.title)
            .map(|(r, g, b)| ratatui::style::Color::Rgb(r, g, b))
            .unwrap_or(ratatui::style::Color::Yellow)
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

impl Default for MpdConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        Self {
            address: "localhost:6600".to_string(),
            music_dir: PathBuf::from(home).join("Music"),
        }
    }
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            border: "#FAE280".to_string(),
            title: "#FAE280".to_string(),
            album: "#FAE280".to_string(),
            artist: "#FAE280".to_string(),
            status: "#FAE280".to_string(),
        }
    }
}
