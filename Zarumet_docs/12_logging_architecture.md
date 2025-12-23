# Logging Architecture

This document explains Zarumet's logging system, which provides structured file-based logging with rotation, platform-specific paths, and helper functions for common log patterns.

## Overview

Zarumet uses the `flexi_logger` crate for logging, providing:

- **File-based logging** - Logs persist to disk for debugging
- **Log rotation** - Automatic size-based rotation with configurable retention
- **Platform-specific paths** - XDG compliance on Linux, appropriate locations on macOS/Windows
- **Structured helpers** - Semantic logging functions for common operations
- **Configurable levels** - Runtime log level selection via config

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Logging Pipeline                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Application Code                                           │
│       │                                                     │
│       ├── log::info!("message")                            │
│       ├── log::debug!("details")                           │
│       └── log_mpd_connection(addr, success, error)         │
│               │                                             │
│               ▼                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              flexi_logger                            │   │
│  │  - Level filtering (error/warn/info/debug/trace)    │   │
│  │  - Custom format (timestamp, level, file:line)       │   │
│  │  - File rotation (size-based)                        │   │
│  └───────────────────────┬─────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Log Files                               │   │
│  │  ~/.local/share/zarumet/logs/zarumet.log            │   │
│  │  ~/.local/share/zarumet/logs/zarumet_2024-12-23...  │   │
│  │  (rotated files)                                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

Logging is configured via the `[logging]` section in `config.toml`:

```toml
[logging]
enabled = true
level = "info"              # error, warn, info, debug, trace
log_to_console = false      # Also print to stdout
append_to_file = true       # Append vs. overwrite
rotate_logs = true          # Enable log rotation
rotation_size_mb = 10       # Rotate when file exceeds this size
keep_log_files = 5          # Number of rotated files to keep
# custom_log_path = "/path/to/custom.log"  # Override default path
```

### Log Levels

| Level | Use Case | Examples |
|-------|----------|----------|
| `error` | Critical failures | Connection failures, data corruption |
| `warn` | Recoverable issues | Failed MPD commands, missing cover art |
| `info` | Normal operation | Startup/shutdown, connections, state changes |
| `debug` | Development detail | Cache hits/misses, timing info |
| `trace` | Verbose tracing | Every function call, full data dumps |

**Note**: In debug builds (`cargo build`), the log level is forced to `debug` regardless of config.

## Implementation

### Logger Initialization

From `src/logging.rs:7-55`:

```rust
pub fn init_logger(config: &LoggingConfig) -> Result<(), FlexiLoggerError> {
    // Debug builds force debug level for development
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

    // Configure file output
    logger = logger
        .log_to_file(
            FileSpec::default()
                .directory(get_log_directory())
                .suppress_timestamp(),
        )
        .format_for_files(custom_log_format)
        .use_utc();

    // Append mode (don't overwrite existing logs)
    if config.append_to_file {
        logger = logger.append();
    }

    // Configure log rotation
    if config.rotate_logs {
        logger = logger.rotate(
            Criterion::Size(config.rotation_size_mb * 1024 * 1024),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(config.keep_log_files as usize),
        );
    }

    // Optional console output
    if config.log_to_console {
        logger = logger.log_to_stdout();
    }

    logger.start()?;
    log::info!("Logger initialized with level: {:?}", log_level);
    Ok(())
}
```

### Custom Log Format

From `src/logging.rs:91-105`:

```rust
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
```

**Output format**:
```
2024-12-23 14:30:45.123 [INFO] [src/app/main_loop.rs:179] Entering event-driven main loop
2024-12-23 14:30:45.456 [DEBUG] [src/app/cover_cache.rs:52] Cover art cache hit: "path/to/song.flac"
```

### Platform-Specific Paths

From `src/logging.rs:57-83`:

```rust
pub fn get_log_directory() -> PathBuf {
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
```

**Default paths**:
| Platform | Log Directory |
|----------|---------------|
| Linux | `~/.local/share/zarumet/logs/` (XDG_DATA_HOME) |
| macOS | `~/Library/Application Support/Logs/zarumet/` |
| Windows | `%APPDATA%\zarumet\logs\` |
| Other | `~/.zarumet/logs/` |

## Semantic Logging Helpers

Zarumet provides helper functions for consistent, structured logging of common operations.

### Startup/Shutdown

```rust
pub fn log_startup_info() {
    log::info!("=== Zarumet Starting ===");
    log::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    log::info!("OS: {}", std::env::consts::OS);
    log::info!("Architecture: {}", std::env::consts::ARCH);
    log::info!("Log file: {}", get_log_file_path().display());
}

pub fn log_shutdown_info() {
    log::info!("=== Zarumet Shutting Down ===");
}
```

### MPD Operations

```rust
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
```

### PipeWire Operations

```rust
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
```

### User Interactions

```rust
pub fn log_user_interaction(action: &str, context: Option<&str>) {
    match context {
        Some(ctx) => log::debug!("User action: {} - {}", action, ctx),
        None => log::debug!("User action: {}", action),
    }
}
```

### Configuration Loading

```rust
pub fn log_config_loading(config_path: &Path, created: bool) {
    if created {
        log::info!("Created default config file at: {}", config_path.display());
    } else {
        log::info!("Loaded config file from: {}", config_path.display());
    }
}
```

## Log Rotation

When enabled, log rotation:

1. **Triggers**: When log file exceeds `rotation_size_mb`
2. **Names rotated files**: With timestamps (e.g., `zarumet_2024-12-23_14-30-45.log`)
3. **Cleans up**: Keeps only `keep_log_files` most recent rotated files

Example directory after rotation:
```
~/.local/share/zarumet/logs/
├── zarumet.log                          # Current log
├── zarumet_2024-12-22_10-00-00.log     # Oldest kept
├── zarumet_2024-12-22_18-30-00.log
├── zarumet_2024-12-23_08-15-00.log
└── zarumet_2024-12-23_12-00-00.log     # Most recent rotated
```

## Usage Examples

### In Application Code

```rust
// Simple logging
log::info!("Starting main loop");
log::debug!("Cache hit rate: {:.1}%", hit_rate * 100.0);
log::warn!("Cover art not found for {:?}", file_path);

// Using helper functions
crate::logging::log_mpd_connection("localhost:6600", true, None);
crate::logging::log_user_interaction("volume_up", Some("new_volume=75"));
```

### Periodic Statistics

The app logs cache statistics periodically:

```rust
// Log every ~600 iterations (about every 30 seconds at 20 FPS)
if counter.is_multiple_of(600) && counter > 0 {
    crate::ui::WIDTH_CACHE.with(|cache| {
        let cache = cache.borrow();
        if cache.total_accesses() > 100 {
            cache.log_stats();
        }
    });
}
```

## Debugging Tips

### Viewing Logs in Real-Time

```bash
# Follow the current log file
tail -f ~/.local/share/zarumet/logs/zarumet.log

# With filtering
tail -f ~/.local/share/zarumet/logs/zarumet.log | grep -E "(ERROR|WARN)"
```

### Increasing Log Verbosity

For debugging, set log level to `debug` or `trace` in config:

```toml
[logging]
level = "debug"
```

Or rebuild in debug mode:
```bash
cargo build  # Debug build forces debug level
```

### Common Log Patterns to Look For

| Pattern | Indicates |
|---------|-----------|
| `Cover art cache hit` | Cache working, no network fetch |
| `Cover art cache miss` | First-time fetch or evicted |
| `MPD command failed` | Connection issues or invalid commands |
| `PipeWire operation failed` | Audio system issues |
| `Width cache hit rate` | Text rendering efficiency |

## Related Documentation

- [Configuration Validation](./10_configuration_validation.md) - Config file format
- [Async I/O Patterns](./04_async_io_patterns.md) - Where logging happens in async code
- [Cover Art Prefetching](./05_cover_art_prefetching.md) - Cache logging details
