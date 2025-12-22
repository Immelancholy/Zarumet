use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "zarumet")]
#[command(author = "Immelancholy")]
#[command(version)]
#[command(about = "A TUI MPD client with album art", long_about = None)]
pub struct Args {
    /// Path to config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// MPD server address (overrides config)
    #[arg(short, long)]
    pub address: Option<String>,

    /// Set bit-perfect mode (PipeWire sample rate matching): "on" or "off"
    #[arg(short, long, value_parser = parse_on_off)]
    pub bit_perfect: Option<bool>,

    /// Generate default config file at the specified path (or ./config.toml if not specified)
    #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = false, default_missing_value = ".")]
    pub generate_config: Option<PathBuf>,
}

/// Parse "on" or "off" string to boolean
fn parse_on_off(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("Invalid value '{}': expected 'on' or 'off'", s)),
    }
}
