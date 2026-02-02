use log::error;
use mpd_client::{Client, commands};

use crate::App;
use crate::app::mpd_handler::MPDAction;
use crate::app::{MenuMode, PanelFocus};
use crate::app::{MessageType, StatusMessage};

/// Trait for navigation-related functionality
pub trait Navigation {
    async fn handle_navigation_action(
        &mut self,
        action: MPDAction,
        client: &Client,
    ) -> color_eyre::Result<()>;
}

impl Navigation for App {
    /// Handle navigation and UI-related actions
    async fn handle_navigation_action(
        &mut self,
        action: MPDAction,
        client: &Client,
    ) -> color_eyre::Result<()> {
        match action {
            MPDAction::QueueUp => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        if !self.queue.is_empty() {
                            let current = self.queue_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.queue_list_state.select(Some(current - 1));
                            } else {
                                // Wrap around to the bottom
                                self.queue_list_state
                                    .select(Some(self.queue.len().saturating_sub(1)));
                            }
                            self.selected_queue_index = self.queue_list_state.selected();
                            self.dirty.mark_queue_selection();
                        }
                    }
                    MenuMode::Artists => {
                        // Navigation is now handled by NavigateUp/Down actions based on panel focus
                    }
                    MenuMode::Albums => {
                        // Navigation is handled by NavigateUp/Down actions based on panel focus
                    }
                }
            }
            MPDAction::QueueDown => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        if !self.queue.is_empty() {
                            let current = self.queue_list_state.selected().unwrap_or(0);
                            if current < self.queue.len().saturating_sub(1) {
                                self.queue_list_state.select(Some(current + 1));
                            } else {
                                // Wrap around to the top
                                self.queue_list_state.select(Some(0));
                            }
                            self.selected_queue_index = self.queue_list_state.selected();
                            self.dirty.mark_queue_selection();
                        }
                    }
                    MenuMode::Artists => {
                        // Navigation is now handled by NavigateUp/Down actions based on panel focus
                    }
                    MenuMode::Albums => {
                        // Navigation is handled by NavigateUp/Down actions based on panel focus
                    }
                }
            }
            MPDAction::PlaySelected => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        // Queue mode: play the selected song
                        if let Some(selected) = self.queue_list_state.selected()
                            && selected < self.queue.len()
                        {
                            let song_position: mpd_client::commands::SongPosition = selected.into();
                            if let Err(e) = client
                                .command(mpd_client::commands::Play::song(song_position))
                                .await
                            {
                                error!("Error playing selected song: {}", e);
                            }
                        }
                    }
                    MenuMode::Albums => {
                        // Albums mode: add selected song to queue (AlbumTracks panel)
                        // Note: In AlbumList panel, binds.rs maps this to SwitchPanelRight
                        self.handle_add_song_in_album_view(client).await?;
                    }
                    MenuMode::Artists => {
                        // Artists mode: handled via ToggleAlbumExpansion in binds.rs
                    }
                }
            }
            MPDAction::MoveUpInQueue => {
                if let Some(selected) = self.queue_list_state.selected()
                    && selected > 0
                    && selected < self.queue.len()
                {
                    // Move song up in queue (from position `selected` to `selected - 1`)
                    let from_pos: mpd_client::commands::SongPosition = selected.into();
                    let to_pos: mpd_client::commands::SongPosition = (selected - 1).into();
                    if let Err(e) = client
                        .command(mpd_client::commands::Move::position(from_pos).to_position(to_pos))
                        .await
                    {
                        error!("Error moving song up in queue: {}", e);
                    } else {
                        // Update selected index to follow the moved song
                        self.queue_list_state.select(Some(selected - 1));
                        self.selected_queue_index = self.queue_list_state.selected();
                        self.dirty.mark_queue();
                    }
                }
            }
            MPDAction::MoveDownInQueue => {
                if let Some(selected) = self.queue_list_state.selected()
                    && selected < self.queue.len().saturating_sub(1)
                {
                    // Move song down in queue (from position `selected` to `selected + 1`)
                    let from_pos: mpd_client::commands::SongPosition = selected.into();
                    let to_pos: mpd_client::commands::SongPosition = (selected + 1).into();
                    if let Err(e) = client
                        .command(mpd_client::commands::Move::position(from_pos).to_position(to_pos))
                        .await
                    {
                        error!("Error moving song down in queue: {}", e);
                    } else {
                        // Update selected index to follow the moved song
                        self.queue_list_state.select(Some(selected + 1));
                        self.selected_queue_index = self.queue_list_state.selected();
                        self.dirty.mark_queue();
                    }
                }
            }

            MPDAction::RemoveFromQueue => {
                if let Some(selected) = self.queue_list_state.selected()
                    && selected < self.queue.len()
                {
                    // Remove the selected song from queue
                    let song_position: mpd_client::commands::SongPosition = selected.into();
                    if let Err(e) = client
                        .command(mpd_client::commands::Delete::position(song_position))
                        .await
                    {
                        error!("Error removing song from queue: {}", e);
                    } else {
                        // Update selected index to stay within bounds
                        if self.queue.is_empty() {
                            self.queue_list_state.select(None);
                        } else if selected >= self.queue.len().saturating_sub(1) {
                            self.queue_list_state
                                .select(Some(self.queue.len().saturating_sub(1)));
                        }
                        self.selected_queue_index = self.queue_list_state.selected();
                        self.dirty.mark_queue();
                    }
                }
            }
            MPDAction::Refresh => {
                // Ignore if already updating
                if self.update_in_progress {
                    log::info!("MPD update already in progress, skipping Refresh action");
                    return Ok(());
                }

                self.user_initiated_reload = true; // Set this when user triggers refresh
                // Trigger MPD database update (equivalent to `mpc update`)
                log::info!("Updating MPD database...");
                match client.command(commands::Update::new()).await {
                    Ok(job_id) => {
                        log::info!("MPD database update started (job {})", job_id);

                        self.library_reload_pending = true;

                        self.pending_artist_index = self.library.as_ref().and_then(|lib| {
                            self.artist_list_state
                                .selected()
                                .and_then(|idx| lib.artists.get(idx).map(|a| a.name.clone()))
                        });

                        self.set_status_message(StatusMessage {
                            text: String::new(),
                            created_at: std::time::Instant::now(),
                            message_type: MessageType::InProgress,
                        })
                    }
                    Err(e) => {
                        error!("Failed to start MPD database update: {}", e);
                        // Continue with library reload anyway
                        //
                        self.set_status_message(StatusMessage {
                            text: String::new(),
                            created_at: std::time::Instant::now(),
                            message_type: MessageType::Error,
                        })
                    }
                }
            }
            MPDAction::SwitchToQueueMenu => {
                // Save current panel focus before leaving
                match self.menu_mode {
                    MenuMode::Artists => self.artists_panel_focus = self.panel_focus.clone(),
                    MenuMode::Albums => self.albums_panel_focus = self.panel_focus.clone(),
                    MenuMode::Queue => {}
                }
                self.menu_mode = MenuMode::Queue;
                self.dirty.mark_menu_mode();
                // Queue mode doesn't use panel focus
            }
            MPDAction::SwitchToArtists => {
                // Save current panel focus before leaving
                match self.menu_mode {
                    MenuMode::Artists => {} // Already in Artists mode
                    MenuMode::Albums => self.albums_panel_focus = self.panel_focus.clone(),
                    MenuMode::Queue => {}
                }
                self.menu_mode = MenuMode::Artists;
                // Restore cached panel focus for Artists mode
                self.panel_focus = self.artists_panel_focus.clone();
                self.dirty.mark_menu_mode();
            }
            MPDAction::SwitchToAlbums => {
                // Save current panel focus before leaving
                match self.menu_mode {
                    MenuMode::Artists => self.artists_panel_focus = self.panel_focus.clone(),
                    MenuMode::Albums => {} // Already in Albums mode
                    MenuMode::Queue => {}
                }
                self.menu_mode = MenuMode::Albums;
                // Restore cached panel focus for Albums mode
                self.panel_focus = self.albums_panel_focus.clone();
                self.dirty.mark_menu_mode();

                self.preload_albums_for_view(client).await;
            }
            MPDAction::SwitchPanelLeft => {
                match self.menu_mode {
                    MenuMode::Artists => {
                        match self.panel_focus {
                            PanelFocus::Artists => {
                                // Already at leftmost panel
                            }
                            PanelFocus::Albums => {
                                self.panel_focus = PanelFocus::Artists;
                                self.dirty.mark_panel_focus();
                                // Preserve album selection when switching to artists panel
                                // (user can return to the same position with SwitchPanelRight)
                            }
                            _ => {
                                // Invalid panel focus for Artists mode, reset to Artists
                                self.panel_focus = PanelFocus::Artists;
                                self.dirty.mark_panel_focus();
                            }
                        }
                    }
                    MenuMode::Albums => {
                        match self.panel_focus {
                            PanelFocus::AlbumList => {
                                // Already at leftmost panel
                            }
                            PanelFocus::AlbumTracks => {
                                self.panel_focus = PanelFocus::AlbumList;
                                self.dirty.mark_panel_focus();
                            }
                            _ => {
                                // Invalid panel focus for Albums mode, reset to AlbumList
                                self.panel_focus = PanelFocus::AlbumList;
                                self.dirty.mark_panel_focus();
                            }
                        }
                    }
                    MenuMode::Queue => {
                        // Queue mode doesn't have panels
                    }
                }
            }
            MPDAction::SwitchPanelRight => {
                match self.menu_mode {
                    MenuMode::Artists => {
                        match self.panel_focus {
                            PanelFocus::Artists => {
                                self.panel_focus = PanelFocus::Albums;
                                self.dirty.mark_panel_focus();
                                // Initialize album selection when switching to albums panel
                                // only if not already set (preserve position on return)
                                if let Some(ref library) = self.library
                                    && let Some(selected_artist_index) =
                                        self.artist_list_state.selected()
                                    && let Some(selected_artist) =
                                        library.get_artist(selected_artist_index)
                                {
                                    // Only initialize if not already selected
                                    if self.album_display_list_state.selected().is_none() {
                                        self.album_display_list_state.select(Some(0));
                                        if !selected_artist.albums.is_empty() {
                                            self.album_list_state.select(Some(0));
                                        }
                                    }
                                }
                            }
                            PanelFocus::Albums => {
                                // Already at rightmost panel
                            }
                            _ => {
                                // Invalid panel focus for Artists mode, reset to Artists
                                self.panel_focus = PanelFocus::Artists;
                                self.dirty.mark_panel_focus();
                            }
                        }
                    }
                    MenuMode::Albums => {
                        match self.panel_focus {
                            PanelFocus::AlbumList => {
                                self.panel_focus = PanelFocus::AlbumTracks;
                                self.dirty.mark_panel_focus();
                                // Initialize track selection when switching to tracks panel
                                if self.album_tracks_list_state.selected().is_none() {
                                    self.album_tracks_list_state.select(Some(0));
                                }
                            }
                            PanelFocus::AlbumTracks => {
                                // Already at rightmost panel
                                self.panel_focus = PanelFocus::AlbumList;
                                self.dirty.mark_panel_focus();
                            }
                            _ => {
                                // Invalid panel focus for Albums mode, reset to AlbumList
                                self.panel_focus = PanelFocus::AlbumList;
                                self.dirty.mark_panel_focus();
                            }
                        }
                    }
                    MenuMode::Queue => {
                        // Queue mode doesn't have panels
                    }
                }
            }
            MPDAction::NavigateUp | MPDAction::NavigateDown => {
                self.handle_panel_navigation(action, client).await;
            }
            MPDAction::GoToTop | MPDAction::GoToBottom => {
                self.handle_go_to_edge(action, client).await;
            }
            MPDAction::ToggleAlbumExpansion => {
                self.handle_album_toggle(client).await?;
            }
            MPDAction::AddSongToQueue => {
                match self.menu_mode {
                    MenuMode::Albums => {
                        // Albums mode: context-aware add
                        match self.panel_focus {
                            PanelFocus::AlbumTracks => {
                                // In tracks panel: add selected song
                                self.handle_add_song_in_album_view(client).await?;
                            }
                            PanelFocus::AlbumList => {
                                // In album list panel: add entire album
                                self.handle_add_album_in_album_view(client).await?;
                            }
                            _ => {}
                        }
                    }
                    MenuMode::Artists => {
                        // Artists mode: context-aware based on what's selected
                        // If on a song, add the song; if on an album, add the album
                        self.handle_add_to_queue_context_aware(client).await?;
                    }
                    MenuMode::Queue => {
                        // Queue mode: no action
                    }
                }
            }
            MPDAction::CycleModeLeft => {
                // Cycle modes left: Queue -> Albums -> Artists -> Queue
                // Save current panel focus before leaving
                match self.menu_mode {
                    MenuMode::Artists => self.artists_panel_focus = self.panel_focus.clone(),
                    MenuMode::Albums => self.albums_panel_focus = self.panel_focus.clone(),
                    MenuMode::Queue => {}
                }
                match self.menu_mode {
                    MenuMode::Queue => {
                        self.menu_mode = MenuMode::Albums;
                        self.panel_focus = self.albums_panel_focus.clone();

                        self.preload_albums_for_view(client).await;
                    }
                    MenuMode::Artists => {
                        self.menu_mode = MenuMode::Queue;
                    }
                    MenuMode::Albums => {
                        self.menu_mode = MenuMode::Artists;
                        self.panel_focus = self.artists_panel_focus.clone();
                    }
                };
                self.dirty.mark_menu_mode();
            }
            MPDAction::CycleModeRight => {
                // Cycle modes right: Queue -> Artists -> Albums -> Queue
                // Save current panel focus before leaving
                match self.menu_mode {
                    MenuMode::Artists => self.artists_panel_focus = self.panel_focus.clone(),
                    MenuMode::Albums => self.albums_panel_focus = self.panel_focus.clone(),
                    MenuMode::Queue => {}
                }
                match self.menu_mode {
                    MenuMode::Queue => {
                        self.menu_mode = MenuMode::Artists;
                        self.panel_focus = self.artists_panel_focus.clone();
                    }
                    MenuMode::Artists => {
                        self.menu_mode = MenuMode::Albums;
                        self.panel_focus = self.albums_panel_focus.clone();

                        self.preload_albums_for_view(client).await;
                    }
                    MenuMode::Albums => {
                        self.menu_mode = MenuMode::Queue;
                    }
                };
                self.dirty.mark_menu_mode();
            }
            MPDAction::ScrollUp | MPDAction::ScrollDown => {
                self.handle_scroll(action, client).await;
            }
            _ => {
                // Execute MPD command for other actions, passing cached status
                if let Err(e) = action
                    .execute(client, &self.config, self.mpd_status.as_ref())
                    .await
                {
                    error!("Error executing MPD command: {}", e);
                }
            }
        }
        Ok(())
    }
}
