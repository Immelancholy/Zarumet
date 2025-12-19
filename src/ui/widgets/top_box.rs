use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::config::Config;
use crate::ui::menu::MenuMode;

pub fn create_top_box<'a>(
    config: &Config,
    mpd_status: Option<&mpd_client::responses::Status>,
    menu_mode: &MenuMode,
) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let text_color = config.colors.song_title_color();
    let accent_color = config.colors.progress_filled_color();

    let mut spans = Vec::new();

    // Playback status indicators
    if let Some(status) = mpd_status {
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
            spans.push(Span::styled("󰮝", Style::default().fg(accent_color).bold()));
        } else {
            spans.push(Span::styled("󰮝", Style::default().fg(text_color)));
        }

        // Playback state and song count
        spans.push(Span::raw("  │  "));
        
        // Queue info
        let queue_count = status.playlist_length;
        let playback_state = match status.state {
            mpd_client::responses::PlayState::Playing => "",
            mpd_client::responses::PlayState::Paused => "",
            mpd_client::responses::PlayState::Stopped => "",
        };
        
        spans.push(Span::styled(playback_state, Style::default().fg(accent_color)));
        spans.push(Span::styled(format!(" {} songs", queue_count), Style::default().fg(text_color)));

        // Volume widget
        spans.push(Span::raw("  │  "));
        
        // Visual volume display with Nerd Font icons
        let volume_bars = status.volume / 10;
        let empty_bars = 10 - volume_bars;
        
        // Volume icon based on level
        let volume_icon = if status.volume == 0 {
            "󰝟"
        } else if status.volume < 33 {
            "󰕿"
        } else if status.volume < 66 {
            "󰖀"
        } else {
            "󰕾"
        };
        
        spans.push(Span::styled(volume_icon, Style::default().fg(text_color)));
        spans.push(Span::styled(" ", Style::default().fg(text_color)));
        spans.push(Span::styled("█".repeat(volume_bars as usize), Style::default().fg(accent_color)));
        spans.push(Span::styled("░".repeat(empty_bars as usize), Style::default().fg(text_color)));
        spans.push(Span::styled(format!(" {}%", status.volume), Style::default().fg(text_color)));
        
        // Menu mode indicator
        spans.push(Span::raw("  │  "));
        let mode_text = match menu_mode {
            MenuMode::Queue => ("󰒺 Queue", accent_color),
            MenuMode::Tracks => ("󰝚 Tracks", accent_color),
        };
        spans.push(Span::styled(mode_text.0, Style::default().fg(mode_text.1)));
    } else {
        spans.push(Span::styled("󰅙 No MPD connection", Style::default().fg(text_color)));
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
