use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, ListState},
};

use crate::config::Config;
use crate::song::{Library, SongInfo};
use crate::ui::menu::{MenuMode, PanelFocus};
use crate::ui::widgets::{
    create_empty_box, create_format_widget, create_left_box_bottom, create_song_widget,
    create_top_box, render_image_widget,
};
use unicode_width::UnicodeWidthStr;

#[allow(clippy::too_many_arguments)]
pub fn render_albums_mode(
    frame: &mut Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    area: Rect,
    format: &Option<String>,
    current_song: &Option<SongInfo>,
    config: &Config,
    library: &Option<Library>,
    artist_list_state: &mut ListState,
    _album_list_state: &mut ListState,
    album_display_list_state: &mut ListState,
    panel_focus: &PanelFocus,
    _expanded_albums: &std::collections::HashSet<(String, String)>,
    play_state: &Option<mpd_client::responses::PlayState>,
    progress: Option<f64>,
    elapsed: Option<std::time::Duration>,
    duration: Option<std::time::Duration>,
    mpd_status: &Option<mpd_client::responses::Status>,
    menu_mode: &MenuMode,
    bit_perfect_enabled: bool,
    skip_image_render: bool,
) {
    // Same layout as tracks mode but for albums
    // Split area vertically: top section, middle section, bottom section
    let main_vertical_chunks = Layout::vertical([
        Constraint::Length(1),       // Format info takes 1 line
        Constraint::Length(3),       // New middle box takes 3 lines
        Constraint::Percentage(100), // Remaining content takes rest
    ])
    .split(area);

    // Split bottom section horizontally: left boxes, right content
    let bottom_horizontal_chunks = Layout::horizontal([
        Constraint::Percentage(50), // Left boxes take 50% of width
        Constraint::Percentage(50), // Right content takes 50% of width
    ])
    .split(main_vertical_chunks[2]);

    // Split left side into two side-by-side boxes and progress bar
    let left_vertical_chunks = Layout::vertical([
        Constraint::Percentage(100), // Two boxes take most of space
        Constraint::Length(3),       // Progress bar takes 3 lines
    ])
    .split(bottom_horizontal_chunks[0]);

    // Split the top part into two side-by-side boxes
    let left_horizontal_chunks = Layout::horizontal([
        Constraint::Percentage(50), // Albums list takes 50% of left space
        Constraint::Percentage(50), // Album tracks take 50% of left space
    ])
    .split(left_vertical_chunks[0]);

    // Render format info widget at top
    let format_widget = create_format_widget(format, current_song, config);
    frame.render_widget(format_widget, main_vertical_chunks[0]);

    // Render middle box that spans both splits
    let middle_box = create_top_box(
        config,
        mpd_status.as_ref(),
        menu_mode,
        bit_perfect_enabled,
        config.pipewire.is_available(),
    );
    frame.render_widget(middle_box, main_vertical_chunks[1]);

    // Render albums list
    if let Some(library) = library {
        let albums_list: Vec<ratatui::widgets::ListItem> = library
            .all_albums
            .iter()
            .map(|(artist_name, album)| {
                // Calculate available width for album name
                let available_width = left_horizontal_chunks[0].width.saturating_sub(4) as usize;

                // Create display text with album name and artist
                let display_text = format!("{} - {}", album.name, artist_name);
                let truncated_text = if display_text.width() > available_width {
                    crate::ui::utils::truncate_by_width(&display_text, available_width)
                } else {
                    display_text
                };

                ratatui::widgets::ListItem::new(vec![Line::from(truncated_text)])
            })
            .collect();

        let albums_border_style = if panel_focus == &PanelFocus::AlbumList {
            Style::default().fg(config.colors.queue_selected_highlight_color())
        } else {
            Style::default().fg(config.colors.border_color())
        };

        let albums_list_widget = ratatui::widgets::List::new(albums_list)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(" Albums ").fg(config.colors.border_title_color()))
                    .border_style(albums_border_style),
            )
            .highlight_style(
                Style::default()
                    .fg(config.colors.queue_selected_text_color())
                    .bg(config.colors.queue_selected_highlight_color()),
            );
        frame.render_stateful_widget(
            albums_list_widget,
            left_horizontal_chunks[0],
            artist_list_state, // Using artist_list_state for album list navigation
        );

        // Show tracks for selected album
        if let Some(selected_album_index) = artist_list_state.selected() {
            if let Some((_artist_name, selected_album)) =
                library.all_albums.get(selected_album_index)
            {
                let tracks_list: Vec<ratatui::widgets::ListItem> = selected_album
                    .tracks
                    .iter()
                    .map(|track| {
                        let track_duration_str = match track.duration {
                            Some(duration) => {
                                let total_seconds = duration.as_secs();
                                let minutes = total_seconds / 60;
                                let seconds = total_seconds % 60;
                                format!("  {}:{:02}", minutes, seconds)
                            }
                            None => "  --:--".to_string(),
                        };

                        let available_width =
                            left_horizontal_chunks[1].width.saturating_sub(4) as usize;
                        let track_duration_width = track_duration_str.width();
                        let max_track_title_width =
                            available_width.saturating_sub(track_duration_width + 3);

                        // Truncate track title if needed to keep duration aligned
                        let truncated_track_title = if track.title.width() > max_track_title_width {
                            crate::ui::utils::truncate_by_width(&track.title, max_track_title_width)
                        } else {
                            track.title.clone()
                        };

                        let filler_width =
                            max_track_title_width.saturating_sub(truncated_track_title.width());
                        let filler = " ".repeat(filler_width.max(0));

                        let track_text = format!("   {}{}", truncated_track_title, filler,);
                        let mut spans = vec![Span::styled(
                            track_text,
                            config.colors.queue_song_title_color(),
                        )];
                        spans.push(Span::styled(
                            track_duration_str.clone(),
                            Style::default().fg(config.colors.track_duration_color()),
                        ));
                        ratatui::widgets::ListItem::new(vec![Line::from(spans)])
                    })
                    .collect();

                let tracks_border_color = if panel_focus == &PanelFocus::AlbumTracks {
                    config.colors.queue_selected_highlight_color()
                } else {
                    config.colors.border_color()
                };

                let tracks_title_color = config.colors.border_title_color();

                // Only show highlight when tracks panel is focused
                let tracks_highlight_style = if panel_focus == &PanelFocus::AlbumTracks {
                    Style::default()
                        .fg(config.colors.queue_selected_text_color())
                        .bg(config.colors.queue_selected_highlight_color())
                } else {
                    Style::default()
                };

                let tracks_list_widget = ratatui::widgets::List::new(tracks_list)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(Line::from(" Tracks ").fg(tracks_title_color))
                            .border_style(Style::default().fg(tracks_border_color)),
                    )
                    .highlight_style(tracks_highlight_style);
                frame.render_stateful_widget(
                    tracks_list_widget,
                    left_horizontal_chunks[1],
                    album_display_list_state, // Using album_display_list_state for tracks navigation
                );
            } else {
                let tracks_box = create_empty_box("Tracks", config);
                frame.render_widget(tracks_box, left_horizontal_chunks[1]);
            }
        } else {
            let tracks_box = create_empty_box("Tracks", config);
            frame.render_widget(tracks_box, left_horizontal_chunks[1]);
        }
    } else {
        let albums_box = create_empty_box("Albums", config);
        frame.render_widget(albums_box, left_horizontal_chunks[0]);
        let tracks_box = create_empty_box("Tracks", config);
        frame.render_widget(tracks_box, left_horizontal_chunks[1]);
    }

    // Render progress bar under the two boxes
    let progress_widget = create_left_box_bottom(play_state, progress, elapsed, duration, config);
    frame.render_widget(progress_widget, left_vertical_chunks[1]);

    // Split the right area vertically: image on top, song info at bottom
    let right_vertical_chunks = Layout::vertical([
        Constraint::Percentage(100), // Image takes most space
        Constraint::Length(4),       // Song info takes 4 lines
    ])
    .split(bottom_horizontal_chunks[1]);

    let image_area = right_vertical_chunks[0];

    // Render image or placeholder
    render_image_widget(frame, protocol, image_area, skip_image_render);

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}
