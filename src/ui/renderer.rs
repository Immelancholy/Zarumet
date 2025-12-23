use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, ListState, Paragraph},
};

use crate::app::MessageType;
use crate::config::Config;
use crate::song::{LazyLibrary, SongInfo};
use crate::ui::ALBUM_DISPLAY_CACHE;
use crate::ui::RENDER_CACHE;
use crate::ui::menu::{MenuMode, PanelFocus};
use crate::ui::utils::DisplayItem;
use crate::ui::widgets::{
    create_empty_box, create_format_widget, create_left_box_bottom, create_left_box_top,
    create_song_widget, create_top_box, render_image_widget,
};
use unicode_width::UnicodeWidthStr;

/// Render status 0n top-right corner (key sequence or status message)
/// Returns true if something was rendered (so loading indicator can skip)
fn render_top_right_status(
    frame: &mut Frame,
    key_binds: &crate::binds::KeyBinds,
    status_message: &Option<crate::app::StatusMessage>,
    area: Rect,
    config: &Config,
) -> bool {
    // Prioritize key sequence if awaiting input
    if key_binds.is_awaiting_input()
        && let Some(text) = get_key_sequence_text(key_binds)
    {
        return render_right_aligned_text(frame, &text, "Seq: ", area, config);
    }

    // Otherwise show status message if present
    if let Some(msg) = status_message
        && let Some(text) = get_status_message_text(msg)
    {
        return render_right_aligned_text(frame, &text, "", area, config);
    }

    false
}

fn get_status_message_text(msg: &crate::app::StatusMessage) -> Option<String> {
    let text = match msg.message_type {
        MessageType::InProgress => "Updating...",
        MessageType::Success => "Updated!",
        MessageType::Error => &msg.text,
    };
    Some(text.to_string())
}

/// Get the current key sequence as displayable text
fn get_key_sequence_text(key_binds: &crate::binds::KeyBinds) -> Option<String> {
    let sequence = key_binds.get_current_sequence();
    if sequence.is_empty() {
        return None;
    }

    // Convert key sequence to display string
    let sequence_text: String = sequence
        .iter()
        .map(|(modifiers, key_code)| {
            let key_str = match key_code {
                crossterm::event::KeyCode::Char(c) => c.to_string(),
                crossterm::event::KeyCode::Esc => "Esc".to_string(),
                crossterm::event::KeyCode::Enter => "Enter".to_string(),
                crossterm::event::KeyCode::Backspace => "Backspace".to_string(),
                crossterm::event::KeyCode::Tab => "Tab".to_string(),
                crossterm::event::KeyCode::Delete => "Delete".to_string(),
                crossterm::event::KeyCode::Insert => "Insert".to_string(),
                crossterm::event::KeyCode::Home => "Home".to_string(),
                crossterm::event::KeyCode::End => "End".to_string(),
                crossterm::event::KeyCode::PageUp => "PageUp".to_string(),
                crossterm::event::KeyCode::PageDown => "PageDown".to_string(),
                crossterm::event::KeyCode::Up => "↑".to_string(),
                crossterm::event::KeyCode::Down => "↓".to_string(),
                crossterm::event::KeyCode::Left => "←".to_string(),
                crossterm::event::KeyCode::Right => "→".to_string(),
                crossterm::event::KeyCode::F(n) => format!("F{}", n),
                _ => format!("{:?}", key_code),
            };

            // Add modifier prefixes
            let mut result = String::new();
            if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                result.push_str("Ctrl+");
            }
            if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
                result.push_str("Alt+");
            }
            if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                result.push_str("Shift+");
            }
            result.push_str(&key_str);
            result
        })
        .collect::<Vec<_>>()
        .join(" → ");
    Some(sequence_text)
}

/// Render right-aligned text within given area
fn render_right_aligned_text(
    frame: &mut Frame,
    text: &str,
    prefix: &str,
    area: Rect,
    config: &Config,
) -> bool {
    let full_text = format!("{}{}", prefix, text);
    let text_width = full_text.width();

    if area.width >= text_width as u16 + 5 && area.height >= 1 {
        let x = area.x + area.width.saturating_sub(text_width as u16 + 5);
        let y = area.y;

        let mut spans = vec![];

        if !prefix.is_empty() {
            spans.push(Span::styled(
                prefix,
                Style::default().fg(config.colors.top_accent_color()),
            ));
        }

        spans.push(Span::styled(
            text,
            Style::default().fg(config.colors.song_title_color()),
        ));

        let line = Line::from(spans);

        frame.render_widget(
            Paragraph::new(line).style(Style::default()),
            Rect {
                x,
                y,
                width: text_width.min(area.width as usize) as u16,
                height: 1,
            },
        );
        return true;
    }
    false
}
/// Render config warnings popup centered on screen
fn render_config_warnings_popup(frame: &mut Frame, warnings: &[String], config: &Config) {
    let area = frame.area();

    // Calculate popup dimensions based on content
    let title = " Unknown Config Options ";
    let footer = "Press any key to close";

    // Find max content width needed
    let max_content_width = warnings
        .iter()
        .map(|w| w.width())
        .max()
        .unwrap_or(20)
        .max(title.width())
        .max(footer.width());

    // Popup width: content + padding (2 on each side) + borders (1 on each side)
    let inner_width = max_content_width + 4;
    let popup_width = (inner_width + 2).min(area.width as usize - 4) as u16;

    // Available width for text inside the popup (subtract borders and padding)
    let text_width = popup_width.saturating_sub(4) as usize;

    // Popup height: warnings + empty line after title + empty line before footer + footer + borders (2)
    let popup_height = (warnings.len() + 5).min(area.height as usize - 4) as u16;

    // Center the popup
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup and fill with background
    frame.render_widget(Clear, popup_area);

    // Build the warning text - truncate if needed
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("")); // Empty line after title

    for warning in warnings {
        // Truncate warning if it's too long
        let display_warning = if warning.width() > text_width {
            let mut truncated = String::new();
            let mut width = 0;
            for c in warning.chars() {
                let c_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if width + c_width + 3 > text_width {
                    truncated.push_str("...");
                    break;
                }
                truncated.push(c);
                width += c_width;
            }
            truncated
        } else {
            warning.clone()
        };

        lines.push(Line::from(Span::styled(
            format!(" {}", display_warning),
            Style::default().fg(config.colors.song_title_color()),
        )));
    }

    lines.push(Line::from("")); // Empty line before footer
    lines.push(
        Line::from(Span::styled(
            footer,
            Style::default().fg(config.colors.top_accent_color()),
        ))
        .centered(),
    );

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(config.colors.queue_selected_highlight_color()))
        .title(Line::from(title).fg(config.colors.border_title_color()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let popup_text = Paragraph::new(lines)
        .block(popup_block)
        .alignment(Alignment::Left);

    frame.render_widget(popup_text, popup_area);
}

/// Renders the user interface.
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    current_song: &Option<SongInfo>,
    queue: &[SongInfo],
    queue_list_state: &mut ListState,
    config: &Config,
    menu_mode: &MenuMode,
    library: &Option<LazyLibrary>,
    artist_list_state: &mut ListState,
    album_list_state: &mut ListState,
    album_display_list_state: &mut ListState,
    all_albums_list_state: &mut ListState,
    album_tracks_list_state: &mut ListState,
    panel_focus: &PanelFocus,
    expanded_albums: &std::collections::HashSet<(String, String)>,
    mpd_status: &Option<mpd_client::responses::Status>,
    key_binds: &crate::binds::KeyBinds,
    bit_perfect_enabled: bool,
    show_config_warnings_popup: bool,
    config_warnings: &[String],
    status_message: &Option<crate::app::StatusMessage>,
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
                bit_perfect_enabled,
                show_config_warnings_popup,
            );
        }
        MenuMode::Artists => {
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
                bit_perfect_enabled,
                show_config_warnings_popup,
            );
        }
        MenuMode::Albums => {
            crate::ui::albums_renderer::render_albums_mode(
                frame,
                protocol,
                area,
                &format,
                current_song,
                config,
                library,
                all_albums_list_state,
                album_tracks_list_state,
                panel_focus,
                expanded_albums,
                &play_state,
                progress,
                elapsed,
                duration,
                mpd_status,
                menu_mode,
                bit_perfect_enabled,
                show_config_warnings_popup,
            );
        }
    }

    // Render key sequence status overlay
    render_top_right_status(frame, key_binds, status_message, area, config);

    // Render config warnings popup if showing
    if show_config_warnings_popup && !config_warnings.is_empty() {
        render_config_warnings_popup(frame, config_warnings, config);
    }
}

#[allow(clippy::too_many_arguments)]
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
    bit_perfect_enabled: bool,
    skip_image_render: bool,
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
    let middle_box = create_top_box(
        config,
        mpd_status.as_ref(),
        menu_mode,
        bit_perfect_enabled,
        config.pipewire.is_available(),
    );
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
    render_image_widget(frame, protocol, image_area, skip_image_render);

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}

#[allow(clippy::too_many_arguments)]
fn render_tracks_mode(
    frame: &mut Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    area: Rect,
    format: &Option<String>,
    current_song: &Option<SongInfo>,
    config: &Config,
    library: &Option<LazyLibrary>,
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
    bit_perfect_enabled: bool,
    skip_image_render: bool,
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
    let middle_box = create_top_box(
        config,
        mpd_status.as_ref(),
        menu_mode,
        bit_perfect_enabled,
        config.pipewire.is_available(),
    );
    frame.render_widget(middle_box, main_vertical_chunks[1]);

    // Render artists list
    if let Some(library) = library {
        let artists_list: Vec<ratatui::widgets::ListItem> = library
            .artists
            .iter()
            .map(|artist| {
                // Calculate available width for artist name (subtract borders and padding)
                let available_width = left_horizontal_chunks[0].width.saturating_sub(4) as usize;
                let truncated_name = crate::ui::WIDTH_CACHE.with(|cache| {
                    let mut cache = cache.borrow_mut();
                    crate::ui::utils::truncate_by_width_cached(
                        &mut cache,
                        &artist.name,
                        available_width,
                    )
                });
                ratatui::widgets::ListItem::new(vec![Line::from(truncated_name)])
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
        if let Some(selected_artist) = library.get_artist(selected_artist_index) {
            // Only initialize album selection if albums panel is focused
            if album_list_state.selected().is_none()
                && panel_focus == &PanelFocus::Albums
                && !selected_artist.albums.is_empty()
            {
                album_list_state.select(Some(0));
            }

            // Use cached display list - get_or_compute returns references,
            // so we clone only the items we need for rendering
            let display_items: Vec<DisplayItem> = ALBUM_DISPLAY_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let (items, _indices) =
                    cache.get_or_compute(selected_artist_index, &selected_artist, expanded_albums);
                items.to_vec()
            });

            let albums_list: Vec<ratatui::widgets::ListItem> = display_items
                .iter()
                .map(|item| {
                    match item {
                        DisplayItem::Album(album_name) => {
                            // Find the actual album to get duration
                            let album = selected_artist
                                .albums
                                .iter()
                                .find(|a| a.name == *album_name)
                                .unwrap();

                            // Format total duration using cache
                            let duration_str =
                                RENDER_CACHE.with(|cache| match album.total_duration() {
                                    Some(duration) => {
                                        let mut cache = cache.borrow_mut();
                                        cache.durations.format_long(duration.as_secs()).to_owned()
                                    }
                                    None => "--:--".to_owned(),
                                });

                            // Calculate available width for filler (subtract album name width and duration width + spaces)
                            let available_width =
                                left_horizontal_chunks[1].width.saturating_sub(4) as usize; // 4 for borders/padding
                            let duration_width = duration_str.width();
                            let max_album_name_width =
                                available_width.saturating_sub(duration_width + 4); // 6 for " " before/after and "     " between name and duration

                            // Truncate album name if needed to keep duration aligned
                            // Note: truncate_by_width_cached pads with spaces, so we trim and calculate filler separately
                            let (truncated_album_name, album_display_width) =
                                crate::ui::WIDTH_CACHE.with(|cache| {
                                    let mut cache = cache.borrow_mut();
                                    let album_width = cache.get_width(album_name);
                                    if album_width <= max_album_name_width {
                                        // Album name fits, use as-is
                                        (album_name.to_string(), album_width)
                                    } else {
                                        // Need to truncate - get truncated version without padding
                                        let truncated = crate::ui::utils::truncate_by_width_cached(
                                            &mut cache,
                                            album_name,
                                            max_album_name_width,
                                        );
                                        let trimmed = truncated.trim_end().to_string();
                                        let width = cache.get_width(&trimmed);
                                        (trimmed, width)
                                    }
                                });

                            let filler_width =
                                max_album_name_width.saturating_sub(album_display_width);
                            let filler = RENDER_CACHE.with(|cache| {
                                cache.borrow().fillers.dashes(filler_width).to_owned()
                            });
                            let display_text =
                                format!(" {}{}   {}", truncated_album_name, filler, duration_str);

                            ratatui::widgets::ListItem::new(vec![
                                Line::from(display_text)
                                    .style(Style::default().fg(config.colors.album_color())),
                            ])
                        }
                        DisplayItem::Song(song_title, duration, _file_path) => {
                            let song_duration_str = RENDER_CACHE.with(|cache| match duration {
                                Some(duration) => {
                                    let mut cache = cache.borrow_mut();
                                    format!(
                                        "  {}",
                                        cache.durations.format_short(duration.as_secs())
                                    )
                                }
                                None => "  --:--".to_owned(),
                            });

                            let available_width =
                                left_horizontal_chunks[1].width.saturating_sub(4) as usize;
                            let song_duration_width = song_duration_str.width();
                            let max_song_title_width =
                                available_width.saturating_sub(song_duration_width + 3); // 3 for "   " prefix

                            // Truncate song title if needed to keep duration aligned
                            let truncated_song_title = crate::ui::WIDTH_CACHE.with(|cache| {
                                let mut cache = cache.borrow_mut();
                                crate::ui::utils::truncate_by_width_cached(
                                    &mut cache,
                                    song_title,
                                    max_song_title_width,
                                )
                            });

                            let filler_width =
                                max_song_title_width.saturating_sub(truncated_song_title.width());
                            let filler = RENDER_CACHE.with(|cache| {
                                cache
                                    .borrow()
                                    .fillers
                                    .spaces(filler_width.max(0))
                                    .to_owned()
                            });

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

            // Only show highlight when albums panel is focused
            let albums_highlight_style = if panel_focus == &PanelFocus::Albums {
                Style::default()
                    .fg(config.colors.queue_selected_text_color())
                    .bg(config.colors.queue_selected_highlight_color())
            } else {
                Style::default()
            };

            let albums_list_widget = ratatui::widgets::List::new(albums_list)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(Line::from(" Albums ").fg(albums_title_color))
                        .border_style(Style::default().fg(albums_border_color)),
                )
                .highlight_style(albums_highlight_style);
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
    render_image_widget(frame, protocol, image_area, skip_image_render);

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}
