use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::Config;
use crate::app::ui::MenuMode;
use crate::app::ui::RENDER_CACHE;

pub fn create_top_box<'a>(
    config: &Config,
    mpd_status: Option<&mpd_client::responses::Status>,
    menu_mode: &MenuMode,
    bit_perfect_enabled: bool,
    bit_perfect_available: bool,
) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let text_color = config.colors.song_title_color();
    let playing = config.colors.playing();
    let paused = config.colors.paused();
    let stopped = config.colors.stopped();
    let accent_color = config.colors.top_accent_color();
    let volume_color = config.colors.volume_color();
    let volume_empty_color = config.colors.volume_empty_color();
    let mode_color = config.colors.mode_color();

    let mut spans = Vec::new();

    // Playback status indicators
    if let Some(status) = mpd_status {
        // Bit-perfect mode (󰤽 - high quality audio icon) - only show if available
        if bit_perfect_available {
            if bit_perfect_enabled {
                spans.push(Span::styled("󰟏", Style::default().fg(accent_color).bold()));
            } else {
                spans.push(Span::styled("󰟏", Style::default().fg(text_color)));
            }
            spans.push(Span::raw(" "));
        }
        // Repeat (󰑖)
        if status.repeat {
            spans.push(Span::styled("󰑖", Style::default().fg(accent_color).bold()));
        } else {
            spans.push(Span::styled("󰑖", Style::default().fg(text_color)));
        }
        spans.push(Span::raw(" "));

        // Random/Shuffle (󰒝)
        if status.random {
            spans.push(Span::styled("󰒝", Style::default().fg(accent_color).bold()));
        } else {
            spans.push(Span::styled("󰒝", Style::default().fg(text_color)));
        }
        spans.push(Span::raw(" "));

        // Single (󰒞)
        if status.single != mpd_client::commands::SingleMode::Disabled {
            spans.push(Span::styled("󰒞", Style::default().fg(accent_color).bold()));
        } else {
            spans.push(Span::styled("󰒞", Style::default().fg(text_color)));
        }
        spans.push(Span::raw(" "));

        // Consume (󰮝)
        if status.consume {
            spans.push(Span::styled("", Style::default().fg(accent_color).bold()));
        } else {
            spans.push(Span::styled("", Style::default().fg(text_color)));
        }
        spans.push(Span::raw(" "));

        // Playback state and song count
        spans.push(Span::raw(" │  "));

        // Queue info
        let queue_count = status.playlist_length;
        let playback_state = match status.state {
            mpd_client::responses::PlayState::Playing => "⏸",
            mpd_client::responses::PlayState::Paused => "▶",
            mpd_client::responses::PlayState::Stopped => "⏹",
        };
        let state_color = match status.state {
            mpd_client::responses::PlayState::Playing => playing,
            mpd_client::responses::PlayState::Paused => paused,
            mpd_client::responses::PlayState::Stopped => stopped,
        };

        spans.push(Span::styled(
            playback_state,
            Style::default().fg(state_color),
        ));
        spans.push(Span::styled(
            format!(" {} songs", queue_count),
            Style::default().fg(text_color),
        ));

        // Volume widget using cached strings
        spans.push(Span::raw("  │  "));

        // Visual volume display with Nerd Font icons
        let volume = status.volume;

        // Volume icon based on level
        let volume_icon = if volume == 0 {
            "󰝟"
        } else if volume < 33 {
            "󰕿"
        } else if volume < 66 {
            "󰖀"
        } else {
            "󰕾"
        };

        // Get cached volume bar strings
        let (filled_str, empty_str, percent_str) = RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            (
                cache.volume_bars.filled(volume).to_owned(),
                cache.volume_bars.empty(volume).to_owned(),
                cache.volume_bars.percent(volume).to_owned(),
            )
        });

        spans.push(Span::styled(volume_icon, Style::default().fg(accent_color)));
        spans.push(Span::styled(" ", Style::default().fg(text_color)));
        spans.push(Span::styled(filled_str, Style::default().fg(volume_color)));
        spans.push(Span::styled(
            empty_str,
            Style::default().fg(volume_empty_color),
        ));
        spans.push(Span::styled(percent_str, Style::default().fg(text_color)));

        // Menu mode indicator
        spans.push(Span::raw("  │  "));
        let mode_text = match menu_mode {
            MenuMode::Queue => (" ", accent_color, "Queue", mode_color),
            MenuMode::Artists => ("󰠃 ", accent_color, "Artists", mode_color),
            MenuMode::Albums => ("󰀥 ", accent_color, "Albums", mode_color),
        };
        spans.push(Span::styled(mode_text.0, Style::default().fg(mode_text.1)));
        spans.push(Span::styled(mode_text.2, Style::default().fg(mode_text.3)));
    } else {
        spans.push(Span::styled(
            "󰅙 No MPD connection",
            Style::default().fg(accent_color),
        ));
    }

    let content = Line::from(spans);

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
