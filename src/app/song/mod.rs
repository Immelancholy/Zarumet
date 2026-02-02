pub mod album;
pub mod artist;
pub mod library;
pub mod song_info;

// Convenience re-exports
pub use album::Album;
pub use artist::{Artist, LazyArtist};
pub use library::LazyLibrary;
pub use song_info::SongInfo;
