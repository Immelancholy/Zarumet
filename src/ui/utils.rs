use crate::ui::width_cache::WidthCache;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use unicode_width::UnicodeWidthChar;

/// Truncate a string to fit within the given display width, handling Unicode properly
pub fn truncate_by_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;

    for ch in s.chars() {
        let char_width = ch.width().unwrap_or(0);
        if current_width + char_width > max_width {
            break;
        }
        result.push(ch);
        current_width += char_width;
    }

    // Pad with spaces if needed
    while current_width < max_width {
        result.push(' ');
        current_width += 1;
    }

    result
}

/// Cached version of truncate_by_width using WidthCache
pub fn truncate_by_width_cached(cache: &mut WidthCache, s: &str, max_width: usize) -> String {
    // For short strings, use the original method (cache overhead not worth it)
    if s.len() < 10 {
        return truncate_by_width(s, max_width);
    }

    // Check if we can use the cached width to avoid full traversal
    if let Some(cached_width) = cache.peek_width(s)
        && cached_width <= max_width {
            // String fits, just pad it
            let mut result = s.to_string();
            let padding_needed = max_width.saturating_sub(cached_width);
            result.push_str(&" ".repeat(padding_needed));
            return result;
        }

    // Fall back to full calculation with caching
    let mut result = String::new();
    let mut current_width = 0;

    for ch in s.chars() {
        let char_width = ch.width().unwrap_or(0);
        if current_width + char_width > max_width {
            break;
        }
        result.push(ch);
        current_width += char_width;
    }

    // Pad with spaces if needed
    while current_width < max_width {
        result.push(' ');
        current_width += 1;
    }

    result
}

/// Left-align a string within given width, handling Unicode properly

/// Cached version of left_align using WidthCache
pub fn left_align_cached(cache: &mut WidthCache, s: &str, width: usize) -> String {
    let display_width = cache.get_width(s);
    if display_width >= width {
        return truncate_by_width_cached(cache, s, width);
    }

    let padding = width - display_width;
    format!("{}{}", s, " ".repeat(padding))
}

/// Format duration as MM:SS
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Helper function to center a rect within another rect
pub fn center_area(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

pub fn center_image(image_dimensions: Rect, available_area: Rect) -> Rect {
    Rect {
        x: available_area.x + (available_area.width - image_dimensions.width) / 2,
        y: available_area.y + (available_area.height - image_dimensions.height) / 2,
        width: image_dimensions.width,
        height: image_dimensions.height,
    }
}

pub struct Protocol {
    pub image: Option<ratatui_image::protocol::StatefulProtocol>,
}

#[derive(Debug, Clone)]
pub enum DisplayItem {
    Album(String),                                                 // album name
    Song(String, Option<std::time::Duration>, std::path::PathBuf), // song title, duration, and file path
}

/// Compute the display list for albums panel considering expanded albums
/// Returns (display_items, mapping_from_display_to_album_index)
pub fn compute_album_display_list(
    artist: &crate::song::Artist,
    expanded_albums: &std::collections::HashSet<(String, String)>,
) -> (Vec<DisplayItem>, Vec<Option<usize>>) {
    let mut display_items = Vec::new();
    let mut album_indices = Vec::new(); // Maps display indices to album indices (None for songs)

    for (album_index, album) in artist.albums.iter().enumerate() {
        let album_key = (artist.name.clone(), album.name.clone());
        let is_expanded = expanded_albums.contains(&album_key);

        // Add album header
        album_indices.push(Some(album_index));
        display_items.push(DisplayItem::Album(album.name.clone()));

        // If expanded, add songs
        if is_expanded {
            for song in &album.tracks {
                album_indices.push(None); // Songs don't map to album indices
                display_items.push(DisplayItem::Song(
                    song.title.clone(),
                    song.duration,
                    song.file_path.clone(),
                ));
            }
        }
    }

    (display_items, album_indices)
}
