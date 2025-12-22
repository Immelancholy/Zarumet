// Module declarations
mod app;
mod binds;
mod config;
mod logging;
#[cfg(target_os = "linux")]
mod pipewire;
mod song;
mod ui;

use app::cli::Args;
use app::{
    App,
    main_loop::AppMainLoop,
    terminal::{init_terminal, restore_terminal},
};
use clap::Parser;
use config::Config;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Handle --generate-config option
    if let Some(path) = &args.generate_config {
        let config_path = if path.is_dir() || path.to_str() == Some(".") {
            path.join("config.toml")
        } else {
            path.clone()
        };
        Config::generate_default(config_path)?;
        return Ok(());
    }

    // Determine config path for logging later
    let config_path = args.config.clone().unwrap_or_else(|| {
        dirs::config_dir()
            .map(|d| d.join("zarumet").join("config.toml"))
            .unwrap_or_default()
    });
    let config_existed = config_path.exists();

    // Load config first for logger initialization
    let mut config = Config::load(args.config.clone())?;

    if let Some(ref addr) = args.address {
        config.mpd.address = addr.clone();
    }

    // Initialize logger first
    if config.logging.enabled {
        crate::logging::ensure_log_directory()?;
        crate::logging::init_logger(&config.logging)?;
        crate::logging::log_startup_info();
        // Log config loading now that logger is initialized
        crate::logging::log_config_loading(&config_path, !config_existed);
    }

    // Initialize terminal
    let terminal = init_terminal()?;

    // Save logging state before app takes ownership
    let logging_enabled = config.logging.enabled;

    // Initialize PipeWire supported rates cache if on Linux and bit-perfect is enabled
    #[cfg(target_os = "linux")]
    {
        if config.pipewire.bit_perfect_enabled
            && let Err(e) = crate::pipewire::initialize_supported_rates()
        {
            log::warn!("Failed to initialize PipeWire supported rates: {}", e);
        }
    }

    // Create app now that logger is initialized
    let app = App::new_with_config(config, args.clone())?;

    // Run application
    let result = app.run(terminal).await;

    // Log shutdown before restoring terminal
    if logging_enabled {
        crate::logging::log_shutdown_info();
    }

    // Restore terminal
    restore_terminal()?;
    result
}
