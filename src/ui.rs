use image::imageops::FilterType;
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage, protocol::StatefulProtocol};

use crate::config::Config;
use crate::song::SongInfo;

pub struct Protocol {
    pub image: Option<StatefulProtocol>,
}

/// Renders the user interface.
pub fn render(
    frame: &mut Frame<'_>,
    protocol: &mut Protocol,
    current_song: &Option<SongInfo>,
    queue: &[SongInfo],
    config: &Config,
) {
    let area = frame.area();

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

    // Render format info widget at top
    let format_widget = create_format_widget(&format, current_song, config);
    frame.render_widget(format_widget, main_vertical_chunks[0]);

    // Render middle box that spans both splits
    let middle_box = create_middle_box(config);
    frame.render_widget(middle_box, main_vertical_chunks[1]);

    // Render widgets in left vertical split
    let left_box_top = create_left_box_top(queue, config, left_vertical_chunks[0]);
    frame.render_widget(left_box_top, left_vertical_chunks[0]);

    // Render widgets in left vertical split
    let left_box_bottom = create_left_box_bottom(&play_state, progress, elapsed, duration, config);
    frame.render_widget(left_box_bottom, left_vertical_chunks[1]);

    // Split the right area vertically: image on top, song info at bottom
    let right_vertical_chunks = Layout::vertical([
        Constraint::Percentage(100), // Image takes most space
        Constraint::Length(4),       // Song info takes 4 lines
    ])
    .split(bottom_horizontal_chunks[1]);

    let image_area = right_vertical_chunks[0];

    // Use full image area for better space utilization

    // Only render image if we have one
    if let Some(ref mut img) = protocol.image {
        // Get the image dimensions after resizing for the available area
        let resize = Resize::Scale(Some(FilterType::Lanczos3));
        let img_rect = img.size_for(resize.clone(), image_area);

        // Center the image within the available area
        let centered_area = center_image(img_rect, image_area);

        let image = StatefulImage::default().resize(resize);
        frame.render_stateful_widget(image, centered_area, img);
    } else {
        let placeholder_area = center_area(
            right_vertical_chunks[0],
            Constraint::Length(12),
            Constraint::Length(1),
        );
        let placeholder = Paragraph::new("No album art").style(Style::default().dark_gray());
        frame.render_widget(placeholder, placeholder_area);
    }

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, right_vertical_chunks[1]);
}

/// Format duration as MM:SS
fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Create the left side box widget
fn create_left_box_bottom(
    play_state: &Option<mpd_client::responses::PlayState>,
    progress: Option<f64>,
    elapsed: Option<std::time::Duration>,
    duration: Option<std::time::Duration>,
    config: &Config,
) -> impl ratatui::widgets::Widget {
    let border_color = config.colors.border_color();
    let border_title_color = config.colors.border_title_color();
    let song_title_color = config.colors.song_title_color();
    let progress_filled_color = config.colors.progress_filled_color();
    let progress_empty_color = config.colors.progress_empty_color();

    let state_text = match play_state {
        Some(mpd_client::responses::PlayState::Playing) => "▶",
        Some(mpd_client::responses::PlayState::Paused) => "⏸",
        Some(mpd_client::responses::PlayState::Stopped) => "⏹",
        None => "⏹",
    };

    let state_color = match play_state {
        Some(mpd_client::responses::PlayState::Playing) => config.colors.playing(),
        Some(mpd_client::responses::PlayState::Paused) => config.colors.paused(),
        Some(mpd_client::responses::PlayState::Stopped) => config.colors.stopped(),
        None => config.colors.stopped(),
    };

    let progress_value = progress.unwrap_or(0.0);
    let progress_percentage = (progress_value * 100.0) as u16;

    struct DynamicProgressBar {
        state_text: String,
        progress_percentage: u16,
        border_title_color: Style,
        border_color: Style,
        song_title_color: Style,
        progress_filled_color: Style,
        progress_empty_color: Style,
        state_color: Style,
        time_elapsed_color: Style,
        time_duration_color: Style,
        time_separator_color: Style,
        elapsed: Option<std::time::Duration>,
        duration: Option<std::time::Duration>,
    }

    impl ratatui::widgets::Widget for DynamicProgressBar {
        fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
            let block = Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(" Progress ", self.border_title_color))
                .border_style(self.border_color);

            let inner = block.inner(area);
            block.render(area, buf);

            // Create styled time spans
            let time_spans = match (self.elapsed, self.duration) {
                (Some(elapsed), Some(duration)) => vec![
                    Span::raw(" "),
                    Span::styled(format_duration(elapsed), self.time_elapsed_color),
                    Span::styled("/", self.time_separator_color),
                    Span::styled(format_duration(duration), self.time_duration_color),
                    Span::raw(" "),
                ],
                (Some(elapsed), None) => vec![
                    Span::raw(" "),
                    Span::styled(format_duration(elapsed), self.time_elapsed_color),
                    Span::styled("/--:--", self.song_title_color),
                    Span::raw(" "),
                ],
                (None, Some(duration)) => vec![
                    Span::raw(" "),
                    Span::styled("--:--/", self.song_title_color),
                    Span::styled(format_duration(duration), self.time_duration_color),
                    Span::raw(" "),
                ],
                (None, None) => vec![Span::styled(" --:--/--:-- ", self.song_title_color)],
            };
            let time_width: usize = time_spans.iter().map(|span| span.content.len()).sum();

            // Calculate available width for progress bar
            let state_width = 3; // state icon
            let spacing_width = 2; // Spaces around percentage
            let total_text_width = state_width + spacing_width + time_width;

            let bar_width = inner.width.saturating_sub(total_text_width as u16) as usize;
            let filled = (self.progress_percentage as usize * bar_width / 100).min(bar_width);
            let empty = bar_width.saturating_sub(filled);

            let mut content_spans = vec![
                Span::styled(&self.state_text, self.state_color),
                Span::styled(" ", self.state_color),
                Span::styled("━".repeat(filled), self.progress_filled_color),
                Span::styled("━".repeat(empty), self.progress_empty_color),
            ];
            content_spans.extend(time_spans);
            let content = Line::from(content_spans);

            let paragraph = Paragraph::new(content).centered();
            paragraph.render(inner, buf);
        }
    }

    DynamicProgressBar {
        state_text: state_text.to_string(),
        progress_percentage,
        border_title_color: Style::default().fg(border_title_color),
        border_color: Style::default().fg(border_color),
        song_title_color: Style::default().fg(song_title_color),
        progress_filled_color: Style::default().fg(progress_filled_color),
        progress_empty_color: Style::default().fg(progress_empty_color),
        state_color: Style::default().fg(state_color),
        time_elapsed_color: Style::default().fg(config.colors.time_elapsed()),
        time_duration_color: Style::default().fg(config.colors.time_duration()),
        time_separator_color: Style::default().fg(config.colors.time_separator()),
        elapsed,
        duration,
    }
}

fn create_left_box_top<'a>(queue: &[SongInfo], config: &Config, area: Rect) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let border_title_color = config.colors.border_title_color();
    let text_color = config.colors.song_title_color();

    // Calculate available width inside the box (minus borders and padding)
    let inner_width = area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding

    let queue_text = if queue.is_empty() {
        "Queue is empty".to_string()
    } else {
        queue
            .iter()
            .take((area.height.saturating_sub(3) as usize).max(1)) // Use full height minus borders/title
            .enumerate()
            .map(|(i, song)| {
                // Calculate dynamic column widths based on available space
                let num_width = 3; // "#. "
                let separator_width = 3; // " | "
                let duration_display_width = 8; // " (MM:SS)"
                let remaining_width = inner_width
                    .saturating_sub(num_width + separator_width * 2 + duration_display_width);

                // Format duration if available
                let duration_str = match song.duration {
                    Some(duration) => {
                        let total_seconds = duration.as_secs();
                        let minutes = total_seconds / 60;
                        let seconds = total_seconds % 60;
                        format!(" ({:02}:{:02})", minutes, seconds)
                    }
                    None => " (--:--)".to_string(),
                };

                // Split remaining width between title, artist, album
                let title_width = (remaining_width * 40) / 100; // 40% for title
                let artist_width = (remaining_width * 25) / 100; // 25% for artist
                let album_width = remaining_width - title_width - artist_width; // 35% for album

                let title = song
                    .title
                    .chars()
                    .take(title_width.max(10))
                    .collect::<String>();
                let artist = song
                    .artist
                    .chars()
                    .take(artist_width.max(8))
                    .collect::<String>();
                let album = song
                    .album
                    .chars()
                    .take(album_width.max(8))
                    .collect::<String>();

                format!(
                    "{}. {:<title_width$} | {:<artist_width$} | {:<album_width$}{}",
                    i + 1,
                    title,
                    artist,
                    album,
                    duration_str,
                    title_width = title_width,
                    artist_width = artist_width,
                    album_width = album_width
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    Paragraph::new(queue_text)
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
        .style(Style::default().fg(text_color))
        .left_aligned()
}

/// Create the middle box widget that spans both splits
fn create_middle_box<'a>(config: &Config) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let text_color = config.colors.song_title_color();

    let content = "Placeholder".to_string();

    Paragraph::new(content)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(text_color))
        .centered()
}

/// Create the format information widget
fn create_format_widget<'a>(
    format: &Option<String>,
    current_song: &Option<SongInfo>,
    config: &Config,
) -> Paragraph<'a> {
    let format_color = config.colors.song_title_color();

    let format_text = match format {
        Some(f) => {
            // Parse format string like "44100:24:2" to extract sample rate
            if let Some(sample_rate_part) = f.split(':').next() {
                if let Ok(sample_rate) = sample_rate_part.parse::<u32>() {
                    // Format sample rate as kHz
                    let sample_rate_khz = sample_rate as f32 / 1000.0;

                    // Extract file extension from the current song's file path
                    let file_type = if let Some(song) = current_song {
                        song.file_path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("unknown")
                            .to_uppercase()
                    } else {
                        "unknown".to_string()
                    };

                    format!("{}: {:.1}kHz", file_type, sample_rate_khz)
                } else {
                    f.clone()
                }
            } else {
                f.clone()
            }
        }
        None => "--".to_string(),
    };

    Paragraph::new(format_text)
        .style(Style::default().fg(format_color))
        .left_aligned()
}

/// Create the song information widget
fn create_song_widget<'a>(current_song: &'a Option<SongInfo>, config: &Config) -> Paragraph<'a> {
    // Get colors from config
    let album_color = config.colors.album_color();
    let artist_color = config.colors.artist_color();
    let song_title_color = config.colors.song_title_color();
    let border_title_color = config.colors.border_title_color();
    let border_color = config.colors.border_color();

    let lines = match current_song {
        Some(song) => {
            vec![
                Line::from(vec![Span::styled(
                    &song.title,
                    Style::default().fg(song_title_color),
                )]),
                Line::from(vec![
                    Span::styled(&song.artist, Style::default().fg(artist_color)),
                    Span::styled(" - ", Style::default().fg(border_title_color)),
                    Span::styled(&song.album, Style::default().fg(album_color)),
                ]),
            ]
        }
        None => vec![Line::from("No song playing").dark_gray()],
    };

    Paragraph::new(lines)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Now Playing ",
                    Style::default().fg(border_title_color),
                ))
                .border_style(Style::default().fg(border_color)),
        )
        .centered()
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
