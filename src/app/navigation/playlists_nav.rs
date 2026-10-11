use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd_client::Client;

use crate::app::mpd::playlists;
use crate::app::mpd_handler::MPDAction;
use crate::app::song::playlist::PlaylistDialog;
use crate::app::{App, MenuMode, MessageType, StatusMessage};

impl App {
    pub async fn handle_playlist_action(&mut self, action: &MPDAction, client: &Client) -> bool {
        if matches!(action, MPDAction::SwitchToPlaylists) {
            match self.menu_mode {
                MenuMode::Artists => self.artists_panel_focus = self.panel_focus.clone(),
                MenuMode::Albums => self.albums_panel_focus = self.panel_focus.clone(),
                MenuMode::Years => self.years_panel_focus = self.panel_focus.clone(),
                MenuMode::Genres => self.genre_panel_focus = self.panel_focus.clone(),
                _ => {}
            }
            self.menu_mode = MenuMode::Playlists;
            self.playlists.invalidate();
            self.dirty.mark_menu_mode();
            return true;
        }
        if matches!(action, MPDAction::SaveQueueAsPlaylist) {
            if matches!(self.menu_mode, MenuMode::Queue | MenuMode::Playlists) {
                self.playlists.dialog = Some(PlaylistDialog::Save {
                    name: String::new(),
                    error: None,
                });
                self.dirty.mark_menu_mode();
            }
            return true;
        }
        if self.menu_mode != MenuMode::Playlists {
            return false;
        }
        match action {
            MPDAction::RenamePlaylist => {
                if let Some(name) = self.playlists.selected_name.clone() {
                    self.playlists.dialog = Some(PlaylistDialog::Rename {
                        original: name.clone(),
                        name,
                        error: None,
                    });
                }
            }
            MPDAction::DeletePlaylist => {
                if let Some(name) = self.playlists.selected_name.clone() {
                    self.playlists.dialog = Some(PlaylistDialog::Delete { name, error: None });
                }
            }
            MPDAction::PlayPlaylist => {
                if let Some(name) = self.playlists.selected_name.clone() {
                    self.playlists.dialog = Some(PlaylistDialog::Play { name });
                }
            }
            MPDAction::AddPlaylistToQueue => {
                if self.playlists.tracks_focused {
                    self.enqueue_playlist_track(client).await;
                } else if let Some(name) = self.playlists.selected_name.clone() {
                    self.enqueue_playlist(client, &name, false).await;
                }
            }
            MPDAction::SwitchPanelLeft => self.playlists.tracks_focused = false,
            MPDAction::SwitchPanelRight => self.playlists.tracks_focused = true,
            MPDAction::Refresh => self.playlists.invalidate(),
            MPDAction::NavigateUp
            | MPDAction::NavigateDown
            | MPDAction::ScrollUp
            | MPDAction::ScrollDown
            | MPDAction::GoToTop
            | MPDAction::GoToBottom => {
                let state = &mut self.playlists;
                let (list, len) = if state.tracks_focused {
                    (&mut state.track_state, state.tracks.len())
                } else {
                    (&mut state.list_state, state.names.len())
                };
                if len > 0 {
                    let current = list.selected().unwrap_or(0).min(len - 1);
                    let index = match action {
                        MPDAction::NavigateUp => (current + len - 1) % len,
                        MPDAction::NavigateDown => (current + 1) % len,
                        MPDAction::ScrollUp => current.saturating_sub(15),
                        MPDAction::ScrollDown => (current + 15).min(len - 1),
                        MPDAction::GoToBottom => len - 1,
                        _ => 0,
                    };
                    if state.tracks_focused {
                        list.select(Some(index));
                    } else {
                        state.select(index);
                    }
                }
            }
            _ => return false,
        }
        self.dirty.mark_menu_mode();
        true
    }

    pub async fn handle_playlist_dialog(&mut self, key: KeyEvent, client: &Client) {
        let Some(mut dialog) = self.playlists.dialog.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {}
            KeyCode::Enter => match dialog {
                PlaylistDialog::Play { name } => self.enqueue_playlist(client, &name, true).await,
                PlaylistDialog::Rename { original, name, .. } => {
                    match playlists::rename(client, &original, &name).await {
                        Ok(()) => {
                            self.playlist_message(format!("Renamed playlist: {name}"), false);
                            self.playlists.selected_name = Some(name);
                            self.playlists.tracks.clear();
                            self.playlists.track_state.select(None);
                        }
                        Err(error) => {
                            self.playlists.dialog = Some(PlaylistDialog::Rename {
                                original,
                                name,
                                error: Some(error.to_string()),
                            });
                        }
                    }
                    self.playlists.invalidate();
                }
                PlaylistDialog::Delete { name, .. } => {
                    match playlists::delete(client, &name).await {
                        Ok(()) => {
                            self.playlist_message(format!("Deleted playlist: {name}"), false);
                            self.playlists.tracks.clear();
                            self.playlists.track_state.select(None);
                        }
                        Err(error) => {
                            self.playlists.dialog = Some(PlaylistDialog::Delete {
                                name,
                                error: Some(error.to_string()),
                            });
                        }
                    }
                    self.playlists.invalidate();
                }
                PlaylistDialog::Save { name, .. } => {
                    match playlists::save_queue(client, &name).await {
                        Ok(()) => {
                            self.playlist_message(format!("Saved playlist: {name}"), false);
                            self.playlists.selected_name = Some(name);
                            self.playlists.tracks.clear();
                            self.playlists.track_state.select(None);
                            self.playlists.invalidate();
                        }
                        Err(error) => {
                            self.playlists.dialog = Some(PlaylistDialog::Save {
                                name,
                                error: Some(error.to_string()),
                            });
                        }
                    }
                }
            },
            _ => {
                if let PlaylistDialog::Save { name, error }
                | PlaylistDialog::Rename { name, error, .. } = &mut dialog
                {
                    match key.code {
                        KeyCode::Backspace => {
                            name.pop();
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            name.clear()
                        }
                        KeyCode::Char(c)
                            if !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                                && !c.is_control() =>
                        {
                            name.push(c)
                        }
                        _ => {}
                    }
                    *error = None;
                }
                self.playlists.dialog = Some(dialog);
            }
        }
        self.dirty.mark_menu_mode();
    }

    async fn enqueue_playlist_track(&mut self, client: &Client) {
        if self.playlists.loading || self.playlists.needs_refresh || self.playlists.error.is_some()
        {
            return;
        }
        let Some(song) = self
            .playlists
            .track_state
            .selected()
            .and_then(|index| self.playlists.tracks.get(index))
            .cloned()
        else {
            return;
        };
        let result = client
            .command(mpd_client::commands::Add::uri(
                &song.file_path.to_string_lossy(),
            ))
            .await;
        match result {
            Ok(_) => self.playlist_message(format!("Added track: {}", song.title), false),
            Err(error) => self.playlist_message(error.to_string(), true),
        }
        self.force_update = true;
    }

    async fn enqueue_playlist(&mut self, client: &Client, name: &str, replace: bool) {
        match playlists::enqueue(client, name, replace).await {
            Ok(count) => self.playlist_message(
                format!(
                    "{} {count} tracks: {name}",
                    if replace { "Playing" } else { "Added" }
                ),
                false,
            ),
            Err(error) => self.playlist_message(error.to_string(), true),
        }
        // Even a failed MPD command may have changed the queue. Always reconcile with MPD.
        self.force_update = true;
        self.playlists.invalidate();
    }

    fn playlist_message(&mut self, text: String, error: bool) {
        if error {
            log::error!("{text}");
        } else {
            log::info!("{text}");
        }
        self.set_status_message(StatusMessage {
            text,
            created_at: std::time::Instant::now(),
            message_type: if error {
                MessageType::Error
            } else {
                MessageType::Success
            },
        });
    }
}
