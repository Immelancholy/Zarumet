pub mod connection;
pub mod cover_load;
pub mod mloop;

pub mod state;

pub use state::check_song_change;

#[cfg(target_os = "linux")]
pub use state::handle_pipewire_state_change;

pub use connection::connect_to_mpd;
pub use cover_load::{CoverArtMessage, spawn_cover_art_loader, spawn_prefetch_loaders};
pub use mloop::AppMainLoop;
