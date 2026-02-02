pub mod generic;
pub mod image;
pub mod progress;
pub mod queue;
pub mod song;
pub mod top_box;

// Re-export all widget functions from separate modules
pub use self::generic::create_empty_box;
pub use self::image::render_image_widget;
pub use self::song::create_format_widget;
pub use self::top_box::create_top_box;

// Legacy function names for backward compatibility
pub use self::progress::create_progress_bar as create_left_box_bottom;
pub use self::queue::create_queue_widget as create_left_box_top;
pub use self::song::create_now_playing_widget as create_song_widget;
