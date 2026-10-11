use mpd_client::Client;

use crate::app::mpd::directories;
use crate::app::mpd_handler::MPDAction;
use crate::app::{App, MenuMode, MessageType, StatusMessage};

impl App {
    pub async fn handle_directory_action(&mut self, action: &MPDAction, client: &Client) -> bool {
        if matches!(action, MPDAction::SwitchToDirectories) {
            match self.menu_mode {
                MenuMode::Artists => self.artists_panel_focus = self.panel_focus.clone(),
                MenuMode::Albums => self.albums_panel_focus = self.panel_focus.clone(),
                MenuMode::Years => self.years_panel_focus = self.panel_focus.clone(),
                MenuMode::Genres => self.genre_panel_focus = self.panel_focus.clone(),
                _ => {}
            }
            self.menu_mode = MenuMode::Directories;
            self.directories.invalidate();
            self.dirty.mark_menu_mode();
            return true;
        }
        if self.menu_mode != MenuMode::Directories {
            return false;
        }
        match action {
            MPDAction::OpenDirectory | MPDAction::AddDirectorySelection => {
                if let Some(entry) = self.directories.focused().selected().cloned() {
                    if entry.is_directory && matches!(action, MPDAction::OpenDirectory) {
                        self.directories.enter(entry.path);
                    } else {
                        let (text, message_type) = match directories::enqueue(client, &entry).await
                        {
                            Ok(()) => (format!("Added: {}", entry.path), MessageType::Success),
                            Err(error) => {
                                log::error!("Could not add {}: {error}", entry.path);
                                (
                                    format!("Could not add: {error}. Check the refreshed queue"),
                                    MessageType::Error,
                                )
                            }
                        };
                        self.set_status_message(StatusMessage {
                            text,
                            created_at: std::time::Instant::now(),
                            message_type,
                        });
                        // MPD may have appended some tracks even if a recursive add failed.
                        self.force_update = true;
                    }
                }
            }
            MPDAction::SwitchPanelLeft => self.directories.preview_focused = false,
            MPDAction::SwitchPanelRight => {
                if !self.directories.preview.path.is_empty() {
                    self.directories.preview_focused = true;
                }
            }
            MPDAction::ParentDirectory => self.directories.parent(),
            MPDAction::Refresh => self.directories.invalidate(),
            MPDAction::NavigateUp
            | MPDAction::NavigateDown
            | MPDAction::ScrollUp
            | MPDAction::ScrollDown
            | MPDAction::GoToTop
            | MPDAction::GoToBottom => {
                let state = self.directories.focused();
                let len = state.entries.len();
                if len > 0 && !state.loading && !state.needs_refresh {
                    let current = state.list_state.selected().unwrap_or(0).min(len - 1);
                    let index = match action {
                        MPDAction::NavigateUp => (current + len - 1) % len,
                        MPDAction::NavigateDown => (current + 1) % len,
                        MPDAction::ScrollUp => current.saturating_sub(15),
                        MPDAction::ScrollDown => (current + 15).min(len - 1),
                        MPDAction::GoToBottom => len - 1,
                        _ => 0,
                    };
                    state.list_state.select(Some(index));
                }
            }
            _ => return false,
        }
        self.directories.sync_preview();
        self.dirty.mark_menu_mode();
        true
    }
}
