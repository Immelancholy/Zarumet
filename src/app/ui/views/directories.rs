use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::song::directory::DirectoryBrowserState;
use crate::app::ui::widgets::{
    create_format_widget, create_left_box_bottom, create_song_widget, create_top_box,
    render_image_widget,
};
use crate::app::ui::{MenuMode, Protocol};
use crate::app::{Config, SongInfo};

#[allow(clippy::too_many_arguments)]
pub fn render_directories_mode(
    frame: &mut Frame<'_>,
    protocol: &mut Protocol,
    current_song: &Option<SongInfo>,
    config: &Config,
    state: &mut DirectoryBrowserState,
    mpd_status: &Option<mpd_client::responses::Status>,
    bit_perfect_enabled: bool,
    skip_image_render: bool,
) {
    // Match the library views: format, status, progress, and current-song artwork.
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(frame.area());
    let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[2]);
    let left = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(columns[0]);
    let lists =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(left[0]);
    let right = Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).split(columns[1]);
    let format = current_song.as_ref().and_then(|song| song.format.clone());
    frame.render_widget(
        create_format_widget(&format, current_song, config),
        vertical[0],
    );
    frame.render_widget(
        create_top_box(
            config,
            mpd_status.as_ref(),
            &MenuMode::Directories,
            bit_perfect_enabled,
            config.pipewire.is_available(),
        ),
        vertical[1],
    );

    let block = |title: String, focused: bool| {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Line::from(title).fg(config.colors.border_title_color()))
            .border_style(Style::default().fg(if focused {
                config.colors.border_selected_color()
            } else {
                config.colors.border_color()
            }))
    };
    let highlight = Style::default()
        .bg(config.colors.queue_selected_highlight_color())
        .fg(config.colors.queue_selected_text_color());
    for (index, pane) in [&mut state.current, &mut state.preview]
        .into_iter()
        .enumerate()
    {
        let focused = if index == 0 {
            !state.preview_focused
        } else {
            state.preview_focused
        };
        let title = if index == 1 && pane.path.is_empty() {
            " Contents ".to_string()
        } else {
            format!(" /{} ", SongInfo::sanitize_string(&pane.path))
        };
        if pane.loading || pane.needs_refresh || pane.error.is_some() || pane.entries.is_empty() {
            let message = if index == 1 && pane.path.is_empty() {
                "Select a directory"
            } else if pane.loading || pane.needs_refresh {
                "Loading…"
            } else if let Some(error) = &pane.error {
                error
            } else {
                "Empty directory"
            };
            frame.render_widget(
                Paragraph::new(message)
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(config.colors.song_title_color()))
                    .block(block(title, focused)),
                lists[index],
            );
        } else {
            let entries: Vec<_> = pane
                .entries
                .iter()
                .map(|entry| {
                    ListItem::new(format!(
                        "{}{}",
                        SongInfo::sanitize_string(entry.name()),
                        if entry.is_directory { "/" } else { "" }
                    ))
                })
                .collect();
            frame.render_stateful_widget(
                List::new(entries)
                    .block(block(title, focused))
                    .style(Style::default().fg(config.colors.song_title_color()))
                    .highlight_style(highlight),
                lists[index],
                &mut pane.list_state,
            );
        }
    }
    let song = current_song.as_ref();
    frame.render_widget(
        create_left_box_bottom(
            &song.and_then(|s| s.play_state),
            song.and_then(|s| s.progress),
            song.and_then(|s| s.elapsed),
            song.and_then(|s| s.duration),
            config,
        ),
        left[1],
    );
    render_image_widget(frame, protocol, right[0], skip_image_render);
    frame.render_widget(create_song_widget(current_song, config), right[1]);
}
