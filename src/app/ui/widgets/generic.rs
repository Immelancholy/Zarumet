use ratatui::{
    style::Style,
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::Config;

pub fn create_empty_box<'a>(title: &'a str, config: &Config) -> Paragraph<'a> {
    let border_color = config.colors.border_color();
    let border_title_color = config.colors.border_title_color();
    let text_color = config.colors.song_title_color();

    Paragraph::new("")
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(border_title_color),
                ))
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(text_color))
        .centered()
}
