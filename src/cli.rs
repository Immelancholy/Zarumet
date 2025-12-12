use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
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

    /// Music directory path (overrides config)
    #[arg(short, long)]
    pub music_dir: Option<PathBuf>,
}

impl Args {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
