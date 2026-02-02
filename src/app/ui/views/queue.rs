use crate::app::{
    ListState, MenuMode,
    config::Config,
    song::SongInfo,
    ui::{
        Protocol,
        widgets::{
            create_format_widget, create_left_box_bottom, create_left_box_top, create_song_widget,
            create_top_box, render_image_widget,
        },
    },
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
};

#[allow(clippy::too_many_arguments)]
pub fn render_queue_mode(
    frame: &mut Frame<'_>,
    protocol: &mut Protocol,
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
