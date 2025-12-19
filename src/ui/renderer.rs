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
use crate::ui::utils::{DisplayItem, compute_album_display_list};
use crate::ui::widgets::{
    create_empty_box, create_format_widget, create_left_box_bottom, create_left_box_top,
    create_song_widget, create_top_box, render_image_widget,
};
use unicode_width::UnicodeWidthStr;

/// Renders the user interface.
pub fn render(
    frame: &mut Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    current_song: &Option<SongInfo>,
    queue: &[SongInfo],
    queue_list_state: &mut ListState,
    config: &Config,
    menu_mode: &MenuMode,
    library: &Option<Library>,
    artist_list_state: &mut ListState,
    album_list_state: &mut ListState,
    album_display_list_state: &mut ListState,
    panel_focus: &PanelFocus,
    expanded_albums: &std::collections::HashSet<(String, String)>,
    mpd_status: &Option<mpd_client::responses::Status>,
) {
    let area = frame.area();

    // Extract play_state, progress, and format from current_song
    let (play_state, progress, elapsed, duration, format) = if let Some(song) = current_song {
        (
            song.play_state,
            song.progress,
            song.elapsed,
            song.duration,
            song.format.clone(),
        )
    } else {
        (None, None, None, None, None)
    };

    match menu_mode {
        MenuMode::Queue => {
            render_queue_mode(
                frame,
                protocol,
                area,
                &format,
                current_song,
                queue,
                queue_list_state,
                config,
                &play_state,
                progress,
                elapsed,
                duration,
                mpd_status,
                menu_mode,
            );
        }
        MenuMode::Tracks => {
            render_tracks_mode(
                frame,
                protocol,
                area,
                &format,
                current_song,
                config,
                library,
                artist_list_state,
                album_list_state,
                album_display_list_state,
                panel_focus,
                expanded_albums,
                &play_state,
                progress,
                elapsed,
                duration,
                mpd_status,
                menu_mode,
            );
        }
    }
}

fn render_queue_mode(
    frame: &mut Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    area: Rect,
    format: &Option<String>,
    current_song: &Option<SongInfo>,
    queue: &[SongInfo],
    queue_list_state: &mut ListState,
    config: &Config,
    play_state: &Option<mpd_client::responses::PlayState>,
    progress: Option<f64>,
    elapsed: Option<std::time::Duration>,
    duration: Option<std::time::Duration>,
    mpd_status: &Option<mpd_client::responses::Status>,
    menu_mode: &MenuMode,
) {
    // Original layout - restore exactly as it was before changes
    // Split the area horizontally: left box, right content
    // Split area vertically: top section, middle section, bottom section
    let main_vertical_chunks = Layout::vertical([
        Constraint::Length(1),       // Format info takes 1 line
        Constraint::Length(3),       // New middle box takes 3 lines
        Constraint::Percentage(100), // Remaining content takes rest
    ])
    .split(area);

    // Split bottom section horizontally: left box, right content
    let bottom_horizontal_chunks = Layout::horizontal([
        Constraint::Percentage(50), // Left box takes 55% of width
        Constraint::Percentage(50), // Right content takes 45% of width
    ])
    .split(main_vertical_chunks[2]);

    let left_vertical_chunks = Layout::vertical([
        Constraint::Percentage(100), // Queue takes most of the space
        Constraint::Length(3),       // Progress bar takes 3 lines
    ])
    .split(bottom_horizontal_chunks[0]);

    // Render format info widget at top
    let format_widget = create_format_widget(format, current_song, config);
    frame.render_widget(format_widget, main_vertical_chunks[0]);

    // Render middle box that spans both splits
    let middle_box = create_top_box(config, mpd_status.as_ref(), menu_mode);
    frame.render_widget(middle_box, main_vertical_chunks[1]);

    // Render widgets in left vertical split
    let left_box_top = create_left_box_top(
        queue,
        queue_list_state,
        current_song,
        config,
        left_vertical_chunks[0],
    );
    frame.render_stateful_widget(left_box_top, left_vertical_chunks[0], queue_list_state);

    // Render widgets in left vertical split
    let left_box_bottom = create_left_box_bottom(play_state, progress, elapsed, duration, config);
    frame.render_widget(left_box_bottom, left_vertical_chunks[1]);

    // Split the right area vertically: image on top, song info at bottom
    let right_vertical_chunks = Layout::vertical([
        Constraint::Percentage(100), // Image takes most space
        Constraint::Length(4),       // Song info takes 4 lines
    ])
    .split(bottom_horizontal_chunks[1]);

    let image_area = right_vertical_chunks[0];

    // Render image or placeholder
    render_image_widget(frame, protocol, image_area);

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}

fn render_tracks_mode(
    frame: &mut Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    area: Rect,
    format: &Option<String>,
    current_song: &Option<SongInfo>,
    config: &Config,
    library: &Option<Library>,
    artist_list_state: &mut ListState,
    album_list_state: &mut ListState,
    album_display_list_state: &mut ListState,
    panel_focus: &PanelFocus,
    expanded_albums: &std::collections::HashSet<(String, String)>,
    play_state: &Option<mpd_client::responses::PlayState>,
    progress: Option<f64>,
    elapsed: Option<std::time::Duration>,
    duration: Option<std::time::Duration>,
    mpd_status: &Option<mpd_client::responses::Status>,
    menu_mode: &MenuMode,
) {
    // Same as original layout, but replace queue box with 2 side-by-side boxes
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
        Constraint::Percentage(100), // Two boxes take most of the space
        Constraint::Length(3),       // Progress bar takes 3 lines
    ])
    .split(bottom_horizontal_chunks[0]);

    // Split the top part into two side-by-side boxes
    let left_horizontal_chunks = Layout::horizontal([
        Constraint::Percentage(35), // Artists box takes 35% of left space
        Constraint::Percentage(65), // Tracks box takes 65% of left space
    ])
    .split(left_vertical_chunks[0]);

    // Render format info widget at top
    let format_widget = create_format_widget(format, current_song, config);
    frame.render_widget(format_widget, main_vertical_chunks[0]);

    // Render middle box that spans both splits
    let middle_box = create_top_box(config, mpd_status.as_ref(), menu_mode);
    frame.render_widget(middle_box, main_vertical_chunks[1]);

    // Render artists list
    if let Some(library) = library {
        let artists_list: Vec<ratatui::widgets::ListItem> = library
            .artists
            .iter()
            .enumerate()
            .map(|(_i, artist)| {
                // Calculate available width for artist name (subtract borders and padding)
                let available_width = left_horizontal_chunks[0].width.saturating_sub(4) as usize;
                let truncated_name = if artist.name.width() > available_width {
                    crate::ui::utils::truncate_by_width(&artist.name, available_width)
                } else {
                    artist.name.clone()
                };
                let display_text = format!("{}", truncated_name);
                ratatui::widgets::ListItem::new(vec![Line::from(display_text)])
            })
            .collect();

        let artists_border_style = if panel_focus == &PanelFocus::Artists {
            Style::default().fg(config.colors.queue_selected_highlight_color())
        } else {
            Style::default().fg(config.colors.border_color())
        };

        let artists_list_widget = ratatui::widgets::List::new(artists_list)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(" Artists ").fg(config.colors.border_title_color()))
                    .border_style(artists_border_style),
            )
            .highlight_style(
                Style::default()
                    .fg(config.colors.queue_selected_text_color())
                    .bg(config.colors.queue_selected_highlight_color()),
            );
        frame.render_stateful_widget(
            artists_list_widget,
            left_horizontal_chunks[0],
            artist_list_state,
        );
    } else {
        let artists_box = create_empty_box("Artists", config);
        frame.render_widget(artists_box, left_horizontal_chunks[0]);
    }

    // Show albums for selected artist, or empty tracks box
    if let (Some(library), Some(selected_artist_index)) = (library, artist_list_state.selected()) {
        if let Some(selected_artist) = library.artists.get(selected_artist_index) {
            // Only initialize album selection if albums panel is focused
            if album_list_state.selected().is_none()
                && panel_focus == &PanelFocus::Albums
                && !selected_artist.albums.is_empty()
            {
                album_list_state.select(Some(0));
            }

            let (display_items, _album_indices) =
                compute_album_display_list(selected_artist, expanded_albums);

            let albums_list: Vec<ratatui::widgets::ListItem> = display_items
                .iter()
                .enumerate()
                .map(|(_display_index, item)| {
                    match item {
                        DisplayItem::Album(album_name) => {
                            // Find the actual album to get duration
                            let album = selected_artist
                                .albums
                                .iter()
                                .find(|a| a.name == *album_name)
                                .unwrap();

                            // Format total duration
                            let duration_str = match album.total_duration() {
                                Some(duration) => {
                                    let total_seconds = duration.as_secs();
                                    let hours = total_seconds / 3600;
                                    let minutes = (total_seconds % 3600) / 60;
                                    let seconds = total_seconds % 60;

                                    if hours > 0 {
                                        format!("{}:{}:{:02}", hours, minutes, seconds)
                                    } else {
                                        format!("{}:{:02}", minutes, seconds)
                                    }
                                }
                                None => "--:--".to_string(),
                            };

                            // Calculate available width for filler (subtract album name width and duration width + spaces)
                            let available_width =
                                left_horizontal_chunks[1].width.saturating_sub(4) as usize; // 4 for borders/padding
                            let duration_width = duration_str.width();
                            let max_album_name_width = available_width.saturating_sub(duration_width + 6); // 6 for " " before/after and "     " between name and duration

                            // Truncate album name if needed to keep duration aligned
                            let truncated_album_name = if album_name.width() > max_album_name_width {
                                crate::ui::utils::truncate_by_width(album_name, max_album_name_width)
                            } else {
                                album_name.clone()
                            };

                            let filler_width = max_album_name_width.saturating_sub(truncated_album_name.width());
                            let filler = "─".repeat(filler_width.max(0));
                            let display_text =
                                format!(" {}{}     {}", truncated_album_name, filler, duration_str);

                            ratatui::widgets::ListItem::new(vec![
                                Line::from(display_text)
                                    .style(Style::default().fg(config.colors.album_color())),
                            ])
                        }
                        DisplayItem::Song(song_title, duration, _file_path) => {
                            let song_duration_str = match duration {
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
                            let song_duration_width = song_duration_str.width();
                            let max_song_title_width = available_width.saturating_sub(song_duration_width + 6); // 6 for "   " prefix and spaces

                            // Truncate song title if needed to keep duration aligned
                            let truncated_song_title = if song_title.width() > max_song_title_width {
                                crate::ui::utils::truncate_by_width(song_title, max_song_title_width)
                            } else {
                                song_title.clone()
                            };

                            let filler_width = max_song_title_width.saturating_sub(truncated_song_title.width());
                            let filler = " ".repeat(filler_width.max(0));

                            let song_text = format!("   {}{}", truncated_song_title, filler,);
                            let mut spans = vec![Span::styled(
                                song_text,
                                config.colors.queue_song_title_color(),
                            )];
                            spans.push(Span::styled(
                                song_duration_str.clone(),
                                Style::default().fg(config.colors.track_duration_color()),
                            ));
                            ratatui::widgets::ListItem::new(vec![Line::from(spans)])
                        }
                    }
                })
                .collect();

            let albums_border_color = if panel_focus == &PanelFocus::Albums {
                config.colors.queue_selected_highlight_color()
            } else {
                config.colors.border_color()
            };

            let albums_title_color = config.colors.border_title_color();

            let albums_list_widget = ratatui::widgets::List::new(albums_list)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(Line::from(" Albums ").fg(albums_title_color))
                        .border_style(Style::default().fg(albums_border_color)),
                )
                .highlight_style(
                    Style::default()
                        .fg(config.colors.queue_selected_text_color())
                        .bg(config.colors.queue_selected_highlight_color()),
                );
            frame.render_stateful_widget(
                albums_list_widget,
                left_horizontal_chunks[1],
                album_display_list_state,
            );
        } else {
            let tracks_box = create_empty_box("Albums", config);
            frame.render_widget(tracks_box, left_horizontal_chunks[1]);
        }
    } else {
        let tracks_box = create_empty_box("Albums", config);
        frame.render_widget(tracks_box, left_horizontal_chunks[1]);
    }

    // Render progress bar under the two empty boxes
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
    render_image_widget(frame, protocol, image_area);

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}
