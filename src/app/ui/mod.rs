pub mod cache;
pub mod rendering;
pub mod views;
pub mod widgets;

pub use cache::DirtyFlags;
pub use cache::RenderCache;
pub use cache::WidthCache;
pub use rendering::{AlbumDisplayCache, DisplayItem, Protocol, compute_album_display_list};
pub use views::{MenuMode, PanelFocus};

use std::cell::RefCell;

thread_local! {
    /// Global width cache for Unicode string width calculations
    /// Using RefCell for interior mutability within single-threaded ratatui context
    pub static WIDTH_CACHE: RefCell<WidthCache> = RefCell::new(WidthCache::new());

    /// Global render cache for expensive string operations
    /// Contains pre-generated fillers, cached durations, volume bars, etc.
    pub static RENDER_CACHE: RefCell<RenderCache> = RefCell::new(RenderCache::new());

    /// Global album display list cache
    /// Caches computed display lists to avoid recomputation each frame
    pub static ALBUM_DISPLAY_CACHE: RefCell<AlbumDisplayCache> = RefCell::new(AlbumDisplayCache::new());
}
