use crate::app::config::LoggingConfig;
use flexi_logger::{Cleanup, Criterion, FileSpec, FlexiLoggerError, Logger, Naming};
use log::LevelFilter;
use std::path::{Path, PathBuf};

/// Initialize the logger for the application
pub fn init_logger(config: &LoggingConfig) -> Result<(), FlexiLoggerError> {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        match config.level.as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    };

    let mut logger = Logger::try_with_str(config.level.to_lowercase())?;

    logger = logger
        .log_to_file(
            FileSpec::default()
                .directory(get_log_directory())
                .suppress_timestamp(),
        )
        .format_for_files(custom_log_format)
        .use_utc();

    if config.append_to_file {
        logger = logger.append();
    }

    // Configure log rotation if enabled
    if config.rotate_logs {
        logger = logger.rotate(
            Criterion::Size(config.rotation_size_mb * 1024 * 1024),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(config.keep_log_files as usize),
        );
    }

    // Only log to console if enabled
    if config.log_to_console {
        logger = logger.log_to_stdout();
    }

    logger.start()?;
    log::info!("Logger initialized with level: {:?}", log_level);
    log::info!("Log file location: {}", get_log_file_path().display());

    Ok(())
}

/// Get the platform-specific log directory
pub fn get_log_directory() -> PathBuf {
    // Use platform-specific directories
    #[cfg(target_os = "linux")]
    return dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/share"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("zarumet/logs");

    #[cfg(target_os = "macos")]
    return dirs::data_dir()
        .map(|h| h.join("Logs/zarumet"))
        .unwrap_or_else(|| PathBuf::from("./logs"));

    #[cfg(target_os = "windows")]
    return dirs::data_dir()
        .map(|d| d.join("zarumet/logs"))
        .unwrap_or_else(|| PathBuf::from("./logs"));

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return dirs::home_dir()
        .map(|h| h.join(".zarumet/logs"))
        .unwrap_or_else(|| PathBuf::from("./logs"));
}

/// Get the full path to the main log file
pub fn get_log_file_path() -> PathBuf {
    get_log_directory().join("zarumet.log")
}

/// Custom log format for file output
fn custom_log_format(
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &log::Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "{} [{}] [{}:{}] {}",
        now.now().format("%Y-%m-%d %H:%M:%S%.3f"),
        record.level(),
        record.file().unwrap_or("unknown"),
        record.line().unwrap_or(0),
        record.args()
    )
}

/// Ensure log directory exists
pub fn ensure_log_directory() -> color_eyre::Result<()> {
    let log_dir = get_log_directory();
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)?;
        log::info!("Created log directory: {}", log_dir.display());
    }
    Ok(())
}

/// Log application startup information
pub fn log_startup_info() {
    log::info!("=== Zarumet Starting ===");
    log::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    log::info!("OS: {}", std::env::consts::OS);
    log::info!("Architecture: {}", std::env::consts::ARCH);
    log::info!("Log file: {}", get_log_file_path().display());
}

/// Log application shutdown information
pub fn log_shutdown_info() {
    log::info!("=== Zarumet Shutting Down ===");
}

/// Log MPD connection attempts
pub fn log_mpd_connection(address: &str, success: bool, error: Option<&str>) {
    if success {
        log::info!("Successfully connected to MPD at: {}", address);
    } else {
        log::error!(
            "Failed to connect to MPD at: {} - {}",
            address,
            error.unwrap_or("Unknown error")
        );
    }
}

/// Log MPD command execution
pub fn log_mpd_command(command: &str, success: bool, error: Option<&str>) {
    if success {
        log::debug!("MPD command executed successfully: {}", command);
    } else {
        log::warn!(
            "MPD command failed: {} - {}",
            command,
            error.unwrap_or("Unknown error")
        );
    }
}

/// Log user interactions for debugging
pub fn log_user_interaction(action: &str, context: Option<&str>) {
    match context {
        Some(ctx) => log::debug!("User action: {} - {}", action, ctx),
        None => log::debug!("User action: {}", action),
    }
}

/// Log configuration loading
pub fn log_config_loading(config_path: &Path, created: bool) {
    if created {
        log::info!("Created default config file at: {}", config_path.display());
    } else {
        log::info!("Loaded config file from: {}", config_path.display());
    }
}

/// Log PipeWire operations
pub fn log_pipewire_operation(operation: &str, success: bool, details: Option<&str>) {
    if success {
        log::debug!("PipeWire operation successful: {}", operation);
    } else {
        log::warn!(
            "PipeWire operation failed: {} - {}",
            operation,
            details.unwrap_or("Unknown error")
        );
    }
}
