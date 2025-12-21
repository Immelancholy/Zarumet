use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::config::Config;
use crate::song::SongInfo;

pub fn create_now_playing_widget<'a>(
    current_song: &'a Option<SongInfo>,
    config: &'a Config,
) -> Paragraph<'a> {
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

pub fn create_format_widget<'a>(
    format: &'a Option<String>,
    current_song: &'a Option<SongInfo>,
    config: &'a Config,
) -> Paragraph<'a> {
    let format_color = config.colors.song_title_color();
    let accent_color = config.colors.top_accent_color();

    let line = match format {
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

                    // Use yellow accent for consistency with sequence display
                    let file_type_span = Span::styled(
                        format!("{}: ", file_type),
                        Style::default().fg(accent_color),
                    );
                    let sample_rate_span = Span::styled(
                        format!("{:.1}kHz", sample_rate_khz),
                        Style::default().fg(format_color),
                    );

                    Line::from(vec![file_type_span, sample_rate_span])
                } else {
                    Line::from(f.clone())
                }
            } else {
                Line::from(f.clone())
            }
        }
        None => Line::from("--"),
    };

    Paragraph::new(line)
        .style(Style::default().fg(format_color))
        .left_aligned()
}
