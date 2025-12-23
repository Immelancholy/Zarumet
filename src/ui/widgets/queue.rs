use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::config::Config;
use crate::song::SongInfo;
use crate::ui::RENDER_CACHE;

pub fn create_queue_widget<'a>(
    queue: &[SongInfo],
    queue_list_state: &ListState,
    current_song: &Option<SongInfo>,
    config: &Config,
    area: Rect,
) -> List<'a> {
    let border_color = config.colors.border_color();
    let border_title_color = config.colors.border_title_color();
    let queue_album_color = config.colors.queue_album_color();
    let queue_artist_color = config.colors.queue_artist_color();
    let queue_song_title_color = config.colors.queue_song_title_color();
    let queue_position_color = config.colors.queue_position_color();
    let queue_duration_color = config.colors.queue_duration_color();

    // Calculate available width inside the box (minus borders and padding)
    let inner_width = area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding

    let queue_items: Vec<ListItem> = if queue.is_empty() {
        vec![]
    } else {
        // First, determine the maximum number width needed for proper alignment
        // Use cached position strings for width calculation
        let max_num_width = RENDER_CACHE.with(|cache| {
            let cache = cache.borrow();
            queue
                .iter()
                .enumerate()
                .take(1000) // Reasonable limit for calculation
                .map(|(i, _)| unicode_width::UnicodeWidthStr::width(cache.queue_positions.get(i)))
                .max()
                .unwrap_or(3) // fallback to 3 for single digit
        });

        queue
            .iter()
            .enumerate()
            .map(|(i, song)| {
                // Calculate available width for entire line using consistent max_num_width
                let separator_width = 3; // " ║ "
                let duration_display_width = 4; // "M:SS"
                let remaining_width = inner_width
                    .saturating_sub(max_num_width + separator_width * 2 + duration_display_width);

                // Split remaining width into 3 equal parts for title, artist, album
                let field_width = remaining_width / 3;

                // Format duration if available using cache
                let duration_str = RENDER_CACHE.with(|cache| match song.duration {
                    Some(duration) => {
                        let mut cache = cache.borrow_mut();
                        cache.durations.format_short(duration.as_secs()).to_owned()
                    }
                    None => " (--:--)".to_owned(),
                });

                // Truncate each field to its allocated width using Unicode-aware width with caching
                let field_width_max = field_width.max(8);
                let (title, artist, album) = crate::ui::WIDTH_CACHE.with(|cache| {
                    let mut cache = cache.borrow_mut();
                    (
                        crate::ui::utils::left_align_cached(
                            &mut cache,
                            &song.title,
                            field_width_max,
                        ),
                        crate::ui::utils::left_align_cached(
                            &mut cache,
                            &song.artist,
                            field_width_max,
                        ),
                        crate::ui::utils::left_align_cached(
                            &mut cache,
                            &song.album,
                            field_width_max,
                        ),
                    )
                });

                // Check if this is the currently playing song
                let is_currently_playing = current_song
                    .as_ref()
                    .map(|current| current.file_path == song.file_path)
                    .unwrap_or(false);

                // Check if this is the selected song
                let is_selected = queue_list_state.selected() == Some(i);

                // Create base style
                let mut queue_album_color = Style::default().fg(queue_album_color);
                let mut queue_song_title_color = Style::default().fg(queue_song_title_color);
                let mut queue_artist_color = Style::default().fg(queue_artist_color);
                let mut border_color = Style::default().fg(border_color);
                let mut duration_color = Style::default().fg(queue_duration_color);
                let mut pos_color = Style::default().fg(queue_position_color);

                // Apply background highlight for selected song
                if is_selected {
                    queue_album_color = queue_album_color
                        .bg(config.colors.queue_selected_highlight_color())
                        .fg(config.colors.queue_selected_text_color());
                    queue_song_title_color = queue_song_title_color
                        .bg(config.colors.queue_selected_highlight_color())
                        .fg(config.colors.queue_selected_text_color());
                    queue_artist_color = queue_artist_color
                        .bg(config.colors.queue_selected_highlight_color())
                        .fg(config.colors.queue_selected_text_color());
                    border_color = border_color
                        .bg(config.colors.queue_selected_highlight_color())
                        .fg(config.colors.queue_selected_text_color());
                    duration_color = duration_color
                        .bg(config.colors.queue_selected_highlight_color())
                        .fg(config.colors.queue_selected_text_color());
                    pos_color = pos_color
                        .bg(config.colors.queue_selected_highlight_color())
                        .fg(config.colors.queue_selected_text_color());
                }

                // Apply bold-italics to currently playing song content
                if is_currently_playing {
                    queue_album_color = queue_album_color.bold().italic();
                    queue_song_title_color = queue_song_title_color.bold().italic();
                    queue_artist_color = queue_artist_color.bold().italic();
                    border_color = border_color.bold().italic();
                    duration_color = duration_color.bold().italic();
                    pos_color = pos_color.bold().italic();
                }

                // Create spans with appropriate styling - use cached position string
                let num_str = RENDER_CACHE.with(|cache| {
                    let cache = cache.borrow();
                    cache.queue_positions.get(i).to_owned()
                });
                let padded_num_str = format!("{:<width$}", num_str, width = max_num_width);
                let mut spans = vec![Span::styled(padded_num_str, pos_color)];

                // Each field should have its own style, but when selected should be overwritten by the selection styling
                spans.push(Span::styled(title.clone(), queue_song_title_color));
                spans.push(Span::styled(" ║ ", border_color));
                spans.push(Span::styled(artist.clone(), queue_artist_color));
                spans.push(Span::styled(" ║ ", border_color));
                spans.push(Span::styled(album.clone(), queue_album_color));
                spans.push(Span::styled(duration_str.clone(), duration_color));

                // If this row is selected, add padding to fill the entire width
                if is_selected {
                    // Calculate the current line width by reconstructing the line content
                    let line_content = format!(
                        "{} ║ {} ║ {}{}",
                        title.clone(),
                        artist.clone(),
                        album.clone(),
                        duration_str.clone()
                    );
                    let current_width =
                        unicode_width::UnicodeWidthStr::width(&line_content as &str)
                            + max_num_width;
                    let remaining_width = area.width.saturating_sub(current_width as u16) as usize;

                    if remaining_width > 0 {
                        // Add spaces to fill the remaining width with the selected background color
                        let padding = RENDER_CACHE.with(|cache| {
                            cache.borrow().fillers.spaces(remaining_width).to_owned()
                        });
                        spans.push(Span::styled(padding, border_color));
                    }
                }

                ListItem::new(Line::from(spans))
            })
            .collect::<Vec<_>>()
    };

    List::new(queue_items)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Queue ",
                    Style::default().fg(border_title_color),
                ))
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(border_color))
        .highlight_style(Style::default())
        .repeat_highlight_symbol(true)
}
