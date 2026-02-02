use crate::app::config::binds::BindsConfig;
use crate::app::config::colors::ColorsConfig;
use crate::app::config::logging::LoggingConfig;
use crate::app::config::mpd::MpdConfig;
use crate::app::config::pipewire::PipewireConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub mpd: MpdConfig,
    #[serde(default)]
    pub colors: ColorsConfig,
    #[serde(default)]
    pub binds: BindsConfig,
    #[serde(default)]
    pub pipewire: PipewireConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use two rows instead of full matrix for memory efficiency
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for (i, a_char) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;

        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)
                .min(curr_row[j] + 1)
                .min(prev_row[j] + cost);
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Find the most similar string from a list of candidates
fn find_similar(unknown: &str, candidates: &[&str]) -> Option<String> {
    let unknown_lower = unknown.to_lowercase();

    // Find the best match based on Levenshtein distance
    let mut best_match: Option<(&str, usize)> = None;

    for &candidate in candidates {
        let distance = levenshtein_distance(&unknown_lower, &candidate.to_lowercase());

        // Only suggest if the distance is reasonable (less than half the length of the longer string)
        let max_len = unknown.len().max(candidate.len());
        let threshold = (max_len / 2).max(3); // At least 3 edits allowed

        if distance <= threshold {
            if let Some((_, best_distance)) = best_match {
                if distance < best_distance {
                    best_match = Some((candidate, distance));
                }
            } else {
                best_match = Some((candidate, distance));
            }
        }
    }

    best_match.map(|(s, _)| s.to_string())
}

/// Format an unknown config warning with optional "did you mean" suggestion
fn format_unknown_warning(section: &str, key: &str, suggestion: Option<&str>) -> String {
    if section == "section" {
        match suggestion {
            Some(s) => format!("Unknown config section: [{}] (did you mean: [{}]?)", key, s),
            None => format!("Unknown config section: [{}]", key),
        }
    } else {
        match suggestion {
            Some(s) => format!(
                "Unknown option in {}: {} (did you mean: {}?)",
                section, key, s
            ),
            None => format!("Unknown option in {}: {}", section, key),
        }
    }
}

impl Config {
    /// Returns the default config file path based on the platform:
    /// - Linux: ~/.config/zarumet/config.toml (XDG_CONFIG_HOME)
    /// - macOS: ~/Library/Application Support/zarumet/config.toml
    /// - Windows: C:\Users\<User>\AppData\Roaming\zarumet\config.toml
    fn default_config_path() -> color_eyre::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine config directory"))?;
        Ok(config_dir.join("zarumet").join("config.toml"))
    }

    pub fn load(config_path: Option<PathBuf>) -> color_eyre::Result<(Self, Vec<String>)> {
        let config_path = match config_path {
            Some(path) => path,
            None => Self::default_config_path()?,
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

            // Note: log_config_logging is called from main.rs after logger is initialized
            eprintln!("Created default config file at: {}", config_path.display());

            return Ok((default_config, Vec::new()));
        }
        let contents = std::fs::read_to_string(&config_path)?;

        // Check for unknown config options before parsing
        let warnings = Self::check_unknown_fields(&contents);

        let config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
            // This warning will be lost since logger isn't initialized yet,
            // but at least we log in debug mode
            if cfg!(debug_assertions) {
                eprintln!("Warning: Failed to parse config file: {}", e);
            }
            Config::default()
        });
        Ok((config, warnings))
    }

    /// Check for unknown fields in the config file and return warnings
    fn check_unknown_fields(contents: &str) -> Vec<String> {
        let mut warnings = Vec::new();

        // Known top-level sections
        const KNOWN_SECTIONS: &[&str] = &["mpd", "colors", "binds", "pipewire", "logging"];

        // Known fields per section
        const KNOWN_MPD_FIELDS: &[&str] = &["address", "volume_increment", "volume_increment_fine"];

        const KNOWN_COLORS_FIELDS: &[&str] = &[
            "border",
            "song_title",
            "album",
            "artist",
            "border_title",
            "progress_filled",
            "progress_empty",
            "paused",
            "playing",
            "stopped",
            "time_separator",
            "time_duration",
            "time_elapsed",
            "queue_selected_highlight",
            "queue_selected_text",
            "queue_album",
            "queue_song_title",
            "queue_artist",
            "queue_position",
            "queue_duration",
            "top_accent",
            "volume",
            "volume_empty",
            "mode",
            "track_duration",
        ];

        const KNOWN_BINDS_FIELDS: &[&str] = &[
            "next",
            "previous",
            "toggle_play_pause",
            "volume_up",
            "volume_up_fine",
            "volume_down",
            "volume_down_fine",
            "toggle_mute",
            "cycle_mode_right",
            "cycle_mode_left",
            "clear_queue",
            "repeat",
            "random",
            "single",
            "consume",
            "quit",
            "refresh",
            "switch_to_queue_menu",
            "switch_to_artists",
            "switch_to_albums",
            "seek_forward",
            "seek_backward",
            "scroll_up",
            "scroll_down",
            "play_selected",
            "remove_from_queue",
            "move_up_in_queue",
            "move_down_in_queue",
            "switch_panel_left",
            "switch_panel_right",
            "toggle_album_expansion",
            "add_to_queue",
            "scroll_up_big",
            "scroll_down_big",
            "go_to_top",
            "go_to_bottom",
            "toggle_bit_perfect",
        ];

        const KNOWN_PIPEWIRE_FIELDS: &[&str] = &["bit_perfect_enabled"];

        const KNOWN_LOGGING_FIELDS: &[&str] = &[
            "enabled",
            "level",
            "log_to_console",
            "append_to_file",
            "rotate_logs",
            "rotation_size_mb",
            "keep_log_files",
            "custom_log_path",
        ];

        // Parse as generic TOML table
        let table: Result<toml::Table, _> = toml::from_str(contents);
        let table = match table {
            Ok(t) => t,
            Err(_) => return warnings, // Let the main parser handle errors
        };

        // Check top-level sections
        for key in table.keys() {
            if !KNOWN_SECTIONS.contains(&key.as_str()) {
                let suggestion = find_similar(key, KNOWN_SECTIONS);
                let msg = format_unknown_warning("section", key, suggestion.as_deref());
                warnings.push(msg);
            }
        }

        // Check fields in each known section
        if let Some(toml::Value::Table(mpd)) = table.get("mpd") {
            for key in mpd.keys() {
                if !KNOWN_MPD_FIELDS.contains(&key.as_str()) {
                    let suggestion = find_similar(key, KNOWN_MPD_FIELDS);
                    let msg = format_unknown_warning("[mpd]", key, suggestion.as_deref());
                    warnings.push(msg);
                }
            }
        }

        if let Some(toml::Value::Table(colors)) = table.get("colors") {
            for key in colors.keys() {
                if !KNOWN_COLORS_FIELDS.contains(&key.as_str()) {
                    let suggestion = find_similar(key, KNOWN_COLORS_FIELDS);
                    let msg = format_unknown_warning("[colors]", key, suggestion.as_deref());
                    warnings.push(msg);
                }
            }
        }

        if let Some(toml::Value::Table(binds)) = table.get("binds") {
            for key in binds.keys() {
                if !KNOWN_BINDS_FIELDS.contains(&key.as_str()) {
                    let suggestion = find_similar(key, KNOWN_BINDS_FIELDS);
                    let msg = format_unknown_warning("[binds]", key, suggestion.as_deref());
                    warnings.push(msg);
                }
            }
        }

        if let Some(toml::Value::Table(pipewire)) = table.get("pipewire") {
            for key in pipewire.keys() {
                if !KNOWN_PIPEWIRE_FIELDS.contains(&key.as_str()) {
                    let suggestion = find_similar(key, KNOWN_PIPEWIRE_FIELDS);
                    let msg = format_unknown_warning("[pipewire]", key, suggestion.as_deref());
                    warnings.push(msg);
                }
            }
        }

        if let Some(toml::Value::Table(logging)) = table.get("logging") {
            for key in logging.keys() {
                if !KNOWN_LOGGING_FIELDS.contains(&key.as_str()) {
                    let suggestion = find_similar(key, KNOWN_LOGGING_FIELDS);
                    let msg = format_unknown_warning("[logging]", key, suggestion.as_deref());
                    warnings.push(msg);
                }
            }
        }

        warnings
    }

    /// Generate a default config file at the specified path
    pub fn generate_default(path: PathBuf) -> color_eyre::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        // Check if file already exists
        if path.exists() {
            return Err(color_eyre::eyre::eyre!(
                "Config file already exists at: {}",
                path.display()
            ));
        }

        // Create default config and serialize
        let default_config = Config::default();
        let toml_string = toml::to_string_pretty(&default_config)?;
        std::fs::write(&path, &toml_string)?;

        println!("Generated default config at: {}", path.display());
        Ok(())
    }
}
