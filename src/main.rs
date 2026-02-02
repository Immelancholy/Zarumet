// Module declarations
mod app;

use app::{
    App, AppMainLoop,
    cli::Args,
    config::Config,
    logging,
    terminal::{init_terminal, restore_terminal},
};
use clap::Parser;

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
    let (mut config, config_warnings) = Config::load(args.config.clone())?;

    if let Some(ref addr) = args.address {
        config.mpd.address = addr.clone();
    }

    // Initialize logger first
    if config.logging.enabled {
        logging::ensure_log_directory()?;
        logging::init_logger(&config.logging)?;
        logging::log_startup_info();
        // Log config loading now that logger is initialized
        logging::log_config_loading(&config_path, !config_existed);

        // Log any config warnings that were collected during loading
        for warning in &config_warnings {
            log::warn!("{}", warning);
        }
    }

    // Initialize terminal
    let terminal = init_terminal()?;

    // Save logging state before app takes ownership
    let logging_enabled = config.logging.enabled;

    // Initialize PipeWire supported rates cache if on Linux and bit-perfect is enabled
    #[cfg(target_os = "linux")]
    {
        if config.pipewire.bit_perfect_enabled
            && let Err(e) = crate::app::audio::pipewire::initialize_supported_rates()
        {
            log::warn!("Failed to initialize PipeWire supported rates: {}", e);
        }
    }

    // Create app now that logger is initialized
    let mut app = App::new_with_config(config, args.clone())?;

    // Set config warnings and show popup if there are any
    if !config_warnings.is_empty() {
        app.config_warnings = config_warnings;
        app.show_config_warnings_popup = true;
    }

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
