use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    /// Enable logging to file
    #[serde(default = "LoggingConfig::default_enabled")]
    pub enabled: bool,
    /// Log level (error, warn, info, debug, trace)
    #[serde(default = "LoggingConfig::default_level")]
    pub level: String,
    /// Enable logging to console
    #[serde(default = "LoggingConfig::default_log_to_console")]
    pub log_to_console: bool,
    /// Append to existing log file
    #[serde(default = "LoggingConfig::default_append_to_file")]
    pub append_to_file: bool,
    /// Enable log rotation
    #[serde(default = "LoggingConfig::default_rotate_logs")]
    pub rotate_logs: bool,
    /// Maximum log file size in MB before rotation
    #[serde(default = "LoggingConfig::default_rotation_size_mb")]
    pub rotation_size_mb: u64,
    /// Number of log files to keep when rotating
    #[serde(default = "LoggingConfig::default_keep_log_files")]
    pub keep_log_files: u32,
    /// Custom log file path (optional)
    #[serde(default)]
    pub custom_log_path: Option<String>,
}

impl LoggingConfig {
    fn default_enabled() -> bool {
        true
    }

    fn default_level() -> String {
        "info".to_string()
    }

    fn default_log_to_console() -> bool {
        false
    }

    fn default_append_to_file() -> bool {
        true
    }

    fn default_rotate_logs() -> bool {
        true
    }

    fn default_rotation_size_mb() -> u64 {
        10
    }

    fn default_keep_log_files() -> u32 {
        5
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: Self::default_enabled(),
            level: Self::default_level(),
            log_to_console: Self::default_log_to_console(),
            append_to_file: Self::default_append_to_file(),
            rotate_logs: Self::default_rotate_logs(),
            rotation_size_mb: Self::default_rotation_size_mb(),
            keep_log_files: Self::default_keep_log_files(),
            custom_log_path: None,
        }
    }
}
