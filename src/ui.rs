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
    config: &Config,
) {
    let area = frame.area();

    // Split the area: image on top, song info at bottom
    let chunks = Layout::vertical([
        Constraint::Min(10),   // Image takes most space
        Constraint::Length(4), // Song info takes 4 lines
    ])
    .split(area);

    let image_area = center_area(
        chunks[0],
        Constraint::Percentage(100),
        Constraint::Percentage(100),
    );

    // Only render image if we have one
    if let Some(ref mut img) = protocol.image {
        let image = StatefulImage::default().resize(Resize::Scale(Some(FilterType::Lanczos3)));
        frame.render_stateful_widget(image, image_area, img);
    } else {
        let centered_area = center_area(image_area, Constraint::Length(12), Constraint::Length(1));

        let placeholder = Paragraph::new("No album art")
            .centered()
            .style(Style::default().dark_gray());
        frame.render_widget(placeholder, centered_area);
    }

    // Render the song information
    let song_widget = create_song_widget(current_song, config);
    frame.render_widget(song_widget, chunks[1]);
}

/// Create the song information widget
fn create_song_widget<'a>(current_song: &'a Option<SongInfo>, config: &Config) -> Paragraph<'a> {
    // Get colors from config
    let album_color = config.colors.album_color();
    let artist_color = config.colors.artist_color();
    let title_color = config.colors.title_color();
    let status_color = config.colors.status_color();
    let border_color = config.colors.border_color();

    let lines = match current_song {
        Some(song) => vec![
            Line::from(vec![Span::styled(
                &song.title,
                Style::default().fg(title_color),
            )]),
            Line::from(vec![
                Span::styled(&song.artist, Style::default().fg(artist_color)),
                Span::styled(" - ", Style::default().fg(status_color)),
                Span::styled(&song.album, Style::default().fg(album_color)),
            ]),
        ],
        None => vec![Line::from("No song playing").dark_gray()],
    };

    Paragraph::new(lines)
        .block(
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Now Playing ",
                    Style::default().fg(status_color),
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
