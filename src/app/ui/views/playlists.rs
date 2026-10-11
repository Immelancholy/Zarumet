use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::app::song::playlist::{PlaylistBrowserState, PlaylistDialog};
use crate::app::ui::widgets::{
    create_format_widget, create_left_box_bottom, create_song_widget, create_top_box,
    render_image_widget,
};
use crate::app::ui::{MenuMode, Protocol};
use crate::app::{Config, SongInfo};

#[allow(clippy::too_many_arguments)]
pub fn render_playlists_mode(
    frame: &mut Frame<'_>,
    protocol: &mut Protocol,
    current_song: &Option<SongInfo>,
    config: &Config,
    state: &mut PlaylistBrowserState,
    mpd_status: &Option<mpd_client::responses::Status>,
    bit_perfect_enabled: bool,
    skip_image_render: bool,
) {
    // Match the library views: format, status, two lists, progress, and current-song artwork.
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
            &MenuMode::Playlists,
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
    let names: Vec<_> = state
        .names
        .iter()
        .map(|name| ListItem::new(SongInfo::sanitize_string(name)))
        .collect();
    let names = List::new(names)
        .block(block(" Playlists ".to_string(), !state.tracks_focused))
        .style(Style::default().fg(config.colors.song_title_color()))
        .highlight_style(highlight);
    frame.render_stateful_widget(names, lists[0], &mut state.list_state);

    let title = format!(" Tracks ({}) ", state.tracks.len());
    if state.loading || state.error.is_some() || state.tracks.is_empty() {
        let message = if state.loading {
            "Loading…"
        } else if let Some(error) = &state.error {
            error
        } else if state.names.is_empty() {
            "No stored playlists. Save the current queue to begin."
        } else {
            "Empty playlist"
        };
        frame.render_widget(
            Paragraph::new(message)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(config.colors.song_title_color()))
                .block(block(title, state.tracks_focused)),
            lists[1],
        );
    } else {
        let tracks: Vec<_> = state
            .tracks
            .iter()
            .enumerate()
            .map(|(index, song)| {
                ListItem::new(format!("{}. {} — {}", index + 1, song.title, song.artist))
            })
            .collect();
        frame.render_stateful_widget(
            List::new(tracks)
                .block(block(title, state.tracks_focused))
                .style(Style::default().fg(config.colors.song_title_color()))
                .highlight_style(highlight),
            lists[1],
            &mut state.track_state,
        );
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
    render_image_widget(
        frame,
        protocol,
        right[0],
        skip_image_render || state.dialog.is_some(),
    );
    frame.render_widget(create_song_widget(current_song, config), right[1]);
}

pub fn render_playlist_dialog(frame: &mut Frame<'_>, dialog: &PlaylistDialog, config: &Config) {
    let area = frame.area();
    let width = area.width.saturating_sub(2).min(68);
    let height = area.height.saturating_sub(2).min(9);
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let (title, text) = match dialog {
        PlaylistDialog::Save { error, .. } => (
            " Save queue as playlist ",
            error
                .as_deref()
                .unwrap_or("Choose a new name. Existing playlists are never overwritten.")
                .to_string(),
        ),
        PlaylistDialog::Rename {
            original, error, ..
        } => (
            " Rename playlist ",
            error.clone().unwrap_or_else(|| {
                format!(
                    "Rename \"{}\". Existing playlists are never overwritten.",
                    SongInfo::sanitize_string(original)
                )
            }),
        ),
        PlaylistDialog::Delete { name, error } => (
            " Delete playlist? ",
            format!(
                "Delete \"{}\"? This cannot be undone.\n{}",
                SongInfo::sanitize_string(name),
                error
                    .as_deref()
                    .unwrap_or("The playback queue and music files are kept.")
            ),
        ),
        PlaylistDialog::Play { name } => (
            " Replace queue and play? ",
            format!(
                "Play \"{}\" instead of the current queue?\nPlayback will start at its first track.",
                SongInfo::sanitize_string(name)
            ),
        ),
    };
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(title).fg(config.colors.border_title_color()))
        .border_style(Style::default().fg(config.colors.border_selected_color()));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(inner);
    if let PlaylistDialog::Save { name, .. } | PlaylistDialog::Rename { name, .. } = dialog {
        let input = format!("{name}▏");
        let scroll = input
            .width()
            .saturating_sub(rows[0].width as usize)
            .min(u16::MAX as usize) as u16;
        frame.render_widget(
            Paragraph::new(input)
                .scroll((0, scroll))
                .style(Style::default().fg(config.colors.song_title_color())),
            rows[0],
        );
    }
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(config.colors.song_title_color())),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(
            if matches!(
                dialog,
                PlaylistDialog::Save { .. } | PlaylistDialog::Rename { .. }
            ) {
                "Enter: confirm   Esc: cancel   Ctrl-U: clear name"
            } else {
                "Enter: confirm   Esc: cancel"
            },
        )
        .style(Style::default().fg(config.colors.border_title_color())),
        rows[2],
    );
}
