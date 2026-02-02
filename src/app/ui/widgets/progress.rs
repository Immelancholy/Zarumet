use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::Config;
use crate::app::ui::RENDER_CACHE;

pub fn create_progress_bar(
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
        Some(mpd_client::responses::PlayState::Playing) => "⏸",
        Some(mpd_client::responses::PlayState::Paused) => "▶",
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

            // Create styled time spans using cached duration strings
            let time_spans = RENDER_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                match (self.elapsed, self.duration) {
                    (Some(elapsed), Some(duration)) => vec![
                        Span::raw(" "),
                        Span::styled(
                            cache.durations.format_short(elapsed.as_secs()).to_owned(),
                            self.time_elapsed_color,
                        ),
                        Span::styled("/", self.time_separator_color),
                        Span::styled(
                            cache.durations.format_short(duration.as_secs()).to_owned(),
                            self.time_duration_color,
                        ),
                        Span::raw(" "),
                    ],
                    (Some(elapsed), None) => vec![
                        Span::raw(" "),
                        Span::styled(
                            cache.durations.format_short(elapsed.as_secs()).to_owned(),
                            self.time_elapsed_color,
                        ),
                        Span::styled("/--:--", self.song_title_color),
                        Span::raw(" "),
                    ],
                    (None, Some(duration)) => vec![
                        Span::raw(" "),
                        Span::styled("--:--/", self.song_title_color),
                        Span::styled(
                            cache.durations.format_short(duration.as_secs()).to_owned(),
                            self.time_duration_color,
                        ),
                        Span::raw(" "),
                    ],
                    (None, None) => vec![Span::styled(" --:--/--:-- ", self.song_title_color)],
                }
            });
            let time_width: usize = time_spans.iter().map(|span| span.content.len()).sum();

            // Calculate available width for progress bar
            let state_width = 3; // state icon
            let spacing_width = 2; // Spaces around percentage
            let total_text_width = state_width + spacing_width + time_width;

            let bar_width = inner.width.saturating_sub(total_text_width as u16) as usize;
            let filled = (self.progress_percentage as usize * bar_width / 100).min(bar_width);
            let empty = bar_width.saturating_sub(filled);

            // Use cached progress bar strings
            let (filled_str, empty_str) = RENDER_CACHE.with(|cache| {
                let cache = cache.borrow();
                (
                    cache.fillers.progress_chars(filled).to_owned(),
                    cache.fillers.progress_chars(empty).to_owned(),
                )
            });

            let mut content_spans = vec![
                Span::styled(&self.state_text, self.state_color),
                Span::styled(" ", self.state_color),
                Span::styled(filled_str, self.progress_filled_color),
                Span::styled(empty_str, self.progress_empty_color),
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
