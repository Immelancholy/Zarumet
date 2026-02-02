use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, ListState, Paragraph},
};

use crate::app::Config;
use crate::app::KeyBinds;
use crate::app::MessageType;
use crate::app::ui::Protocol;
use crate::app::ui::views::{
    albums::render_albums_mode, artists::render_artists_mode, queue::render_queue_mode,
};
use crate::app::{LazyLibrary, SongInfo};
use crate::app::{MenuMode, PanelFocus};
use unicode_width::UnicodeWidthStr;

/// Render status 0n top-right corner (key sequence or status message)
/// Returns true if something was rendered (so loading indicator can skip)
fn render_top_right_status(
    frame: &mut Frame,
    key_binds: &KeyBinds,
    status_message: &Option<crate::app::StatusMessage>,
    area: Rect,
    config: &Config,
) -> bool {
    // Prioritize key sequence if awaiting input
    if key_binds.is_awaiting_input()
        && let Some(text) = get_key_sequence_text(key_binds)
    {
        return render_right_aligned_text(frame, &text, "Seq: ", area, config);
    }

    // Otherwise show status message if present
    if let Some(msg) = status_message
        && let Some(text) = get_status_message_text(msg)
    {
        return render_right_aligned_text(frame, &text, "", area, config);
    }

    false
}

fn get_status_message_text(msg: &crate::app::StatusMessage) -> Option<String> {
    let text = match msg.message_type {
        MessageType::InProgress => {
            // Animate dots: "Updating." → "Updating.." → "Updating..." → repeat
            let elapsed_ms = msg.created_at.elapsed().as_millis() as u64;
            let frame = (elapsed_ms / 500) % 3; // Cycle through 0, 1, 2
            match frame {
                0 => "Updating.  ",
                1 => "Updating.. ",
                _ => "Updating...",
            }
        }
        MessageType::Success => "Updated!  ",
        MessageType::Error => &msg.text,
    };
    Some(text.to_string())
}

/// Get the current key sequence as displayable text
fn get_key_sequence_text(key_binds: &KeyBinds) -> Option<String> {
    let sequence = key_binds.get_current_sequence();
    if sequence.is_empty() {
        return None;
    }

    // Convert key sequence to display string
    let sequence_text: String = sequence
        .iter()
        .map(|(modifiers, key_code)| {
            let key_str = match key_code {
                crossterm::event::KeyCode::Char(c) => c.to_string(),
                crossterm::event::KeyCode::Esc => "Esc".to_string(),
                crossterm::event::KeyCode::Enter => "Enter".to_string(),
                crossterm::event::KeyCode::Backspace => "Backspace".to_string(),
                crossterm::event::KeyCode::Tab => "Tab".to_string(),
                crossterm::event::KeyCode::Delete => "Delete".to_string(),
                crossterm::event::KeyCode::Insert => "Insert".to_string(),
                crossterm::event::KeyCode::Home => "Home".to_string(),
                crossterm::event::KeyCode::End => "End".to_string(),
                crossterm::event::KeyCode::PageUp => "PageUp".to_string(),
                crossterm::event::KeyCode::PageDown => "PageDown".to_string(),
                crossterm::event::KeyCode::Up => "↑".to_string(),
                crossterm::event::KeyCode::Down => "↓".to_string(),
                crossterm::event::KeyCode::Left => "←".to_string(),
                crossterm::event::KeyCode::Right => "→".to_string(),
                crossterm::event::KeyCode::F(n) => format!("F{}", n),
                _ => format!("{:?}", key_code),
            };

            // Add modifier prefixes
            let mut result = String::new();
            if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                result.push_str("Ctrl+");
            }
            if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
                result.push_str("Alt+");
            }
            if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                result.push_str("Shift+");
            }
            result.push_str(&key_str);
            result
        })
        .collect::<Vec<_>>()
        .join(" → ");
    Some(sequence_text)
}

/// Render right-aligned text within given area
fn render_right_aligned_text(
    frame: &mut Frame,
    text: &str,
    prefix: &str,
    area: Rect,
    config: &Config,
) -> bool {
    let full_text = format!("{}{}", prefix, text);
    let text_width = full_text.width();

    if area.width >= text_width as u16 + 5 && area.height >= 1 {
        let x = area.x + area.width.saturating_sub(text_width as u16 + 5);
        let y = area.y;

        let mut spans = vec![];

        if !prefix.is_empty() {
            spans.push(Span::styled(
                prefix,
                Style::default().fg(config.colors.top_accent_color()),
            ));
        }

        spans.push(Span::styled(
            text,
            Style::default().fg(config.colors.song_title_color()),
        ));

        let line = Line::from(spans);

        frame.render_widget(
            Paragraph::new(line).style(Style::default()),
            Rect {
                x,
                y,
                width: text_width.min(area.width as usize) as u16,
                height: 1,
            },
        );
        return true;
    }
    false
}
/// Render config warnings popup centered on screen
fn render_config_warnings_popup(frame: &mut Frame, warnings: &[String], config: &Config) {
    let area = frame.area();

    // Calculate popup dimensions based on content
    let title = " Unknown Config Options ";
    let footer = "Press any key to close";

    // Find max content width needed
    let max_content_width = warnings
        .iter()
        .map(|w| w.width())
        .max()
        .unwrap_or(20)
        .max(title.width())
        .max(footer.width());

    // Popup width: content + padding (2 on each side) + borders (1 on each side)
    let inner_width = max_content_width + 4;
    let popup_width = (inner_width + 2).min(area.width as usize - 4) as u16;

    // Available width for text inside the popup (subtract borders and padding)
    let text_width = popup_width.saturating_sub(4) as usize;

    // Popup height: warnings + empty line after title + empty line before footer + footer + borders (2)
    let popup_height = (warnings.len() + 5).min(area.height as usize - 4) as u16;

    // Center the popup
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup and fill with background
    frame.render_widget(Clear, popup_area);

    // Build the warning text - truncate if needed
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("")); // Empty line after title

    for warning in warnings {
        // Truncate warning if it's too long
        let display_warning = if warning.width() > text_width {
            let mut truncated = String::new();
            let mut width = 0;
            for c in warning.chars() {
                let c_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if width + c_width + 3 > text_width {
                    truncated.push_str("...");
                    break;
                }
                truncated.push(c);
                width += c_width;
            }
            truncated
        } else {
            warning.clone()
        };

        lines.push(Line::from(Span::styled(
            format!(" {}", display_warning),
            Style::default().fg(config.colors.song_title_color()),
        )));
    }

    lines.push(Line::from("")); // Empty line before footer
    lines.push(
        Line::from(Span::styled(
            footer,
            Style::default().fg(config.colors.top_accent_color()),
        ))
        .centered(),
    );

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(config.colors.queue_selected_highlight_color()))
        .title(Line::from(title).fg(config.colors.border_title_color()))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let popup_text = Paragraph::new(lines)
        .block(popup_block)
        .alignment(Alignment::Left);

    frame.render_widget(popup_text, popup_area);
}

/// Renders the user interface.
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame<'_>,
    protocol: &mut Protocol,
    current_song: &Option<SongInfo>,
    queue: &[SongInfo],
    queue_list_state: &mut ListState,
    config: &Config,
    menu_mode: &MenuMode,
    library: &Option<LazyLibrary>,
    artist_list_state: &mut ListState,
    album_list_state: &mut ListState,
    album_display_list_state: &mut ListState,
    all_albums_list_state: &mut ListState,
    album_tracks_list_state: &mut ListState,
    panel_focus: &PanelFocus,
    expanded_albums: &std::collections::HashSet<(String, String)>,
    mpd_status: &Option<mpd_client::responses::Status>,
    key_binds: &KeyBinds,
    bit_perfect_enabled: bool,
    show_config_warnings_popup: bool,
    config_warnings: &[String],
    status_message: &Option<crate::app::StatusMessage>,
) {
    let area = frame.area();

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

    match menu_mode {
        MenuMode::Queue => {
            render_queue_mode(
                frame,
                protocol,
                area,
                &format,
                current_song,
                queue,
                queue_list_state,
                config,
                &play_state,
                progress,
                elapsed,
                duration,
                mpd_status,
                menu_mode,
                bit_perfect_enabled,
                show_config_warnings_popup,
            );
        }
        MenuMode::Artists => {
            render_artists_mode(
                frame,
                protocol,
                area,
                &format,
                current_song,
                config,
                library,
                artist_list_state,
                album_list_state,
                album_display_list_state,
                panel_focus,
                expanded_albums,
                &play_state,
                progress,
                elapsed,
                duration,
                mpd_status,
                menu_mode,
                bit_perfect_enabled,
                show_config_warnings_popup,
            );
        }
        MenuMode::Albums => {
            render_albums_mode(
                frame,
                protocol,
                area,
                &format,
                current_song,
                config,
                library,
                all_albums_list_state,
                album_tracks_list_state,
                panel_focus,
                expanded_albums,
                &play_state,
                progress,
                elapsed,
                duration,
                mpd_status,
                menu_mode,
                bit_perfect_enabled,
                show_config_warnings_popup,
            );
        }
    }

    // Render key sequence status overlay
    render_top_right_status(frame, key_binds, status_message, area, config);

    // Render config warnings popup if showing
    if show_config_warnings_popup && !config_warnings.is_empty() {
        render_config_warnings_popup(frame, config_warnings, config);
    }
}
