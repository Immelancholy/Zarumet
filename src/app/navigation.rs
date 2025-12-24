use log::error;
use mpd_client::{Client, commands};

use super::App;
use crate::app::mpd_handler::MPDAction;
use crate::app::{MessageType, StatusMessage};
use crate::ui::menu::{MenuMode, PanelFocus};
use crate::ui::{DisplayItem, compute_album_display_list};

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

impl App {
    /// Preload all albums for Albums view and initialize selection
    async fn preload_albums_for_view(&mut self, client: &Client) {
        if let Some(ref mut library) = self.library
            && !library.all_albums_complete
        {
            log::info!("Preloading all albums for Albums view...");
            if let Err(e) = library.preload_all_albums(client).await {
                log::warn!("Failed to preload all albums: {}", e);
            }
        }

        if let Some(ref mut library) = self.library {
            library.ensure_albums_sorted();
        }

        if let Some(ref library) = self.library
            && !library.all_albums.is_empty()
            && self.all_albums_list_state.selected().is_none()
        {
            self.all_albums_list_state.select(Some(0));
        }
    }

    /// Handle panel-specific navigation
    async fn handle_panel_navigation(&mut self, action: MPDAction, client: &Client) {
        match action {
            MPDAction::NavigateUp => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        // Queue navigation is handled elsewhere
                    }
                    MenuMode::Artists => {
                        match self.panel_focus {
                            PanelFocus::Artists => {
                                // Navigate artists list
                                if let Some(ref library) = self.library
                                    && !library.artists.is_empty()
                                {
                                    let current = self.artist_list_state.selected().unwrap_or(0);
                                    let new_index = if current > 0 {
                                        current - 1
                                    } else {
                                        // Wrap around to the bottom
                                        library.artists.len().saturating_sub(1)
                                    };
                                    self.artist_list_state.select(Some(new_index));
                                    // Clear album selection when navigating artists
                                    self.album_list_state.select(None);
                                    self.album_display_list_state.select(None);

                                    // Lazy load the newly selected artist's albums
                                    if let Some(ref mut library) = self.library
                                        && let Err(e) = library.load_artist(client, new_index).await
                                    {
                                        log::warn!("Failed to load artist: {}", e);
                                    }
                                }
                            }
                            PanelFocus::Albums => {
                                // Navigate albums list using display list state
                                if let (Some(library), Some(selected_artist_index)) =
                                    (&self.library, self.artist_list_state.selected())
                                    && let Some(selected_artist) =
                                        library.get_artist(selected_artist_index)
                                {
                                    // Compute display list to get total count
                                    let (display_items, _album_indices) =
                                        compute_album_display_list(
                                            &selected_artist,
                                            &self.expanded_albums,
                                        );

                                    if !display_items.is_empty() {
                                        let current =
                                            self.album_display_list_state.selected().unwrap_or(0);
                                        if current > 0 {
                                            self.album_display_list_state.select(Some(current - 1));
                                        } else {
                                            // Wrap around to bottom
                                            self.album_display_list_state.select(Some(
                                                display_items.len().saturating_sub(1),
                                            ));
                                        }

                                        // Update the legacy album_list_state to point to the current album if on album
                                        let wrapped_index = if current > 0 {
                                            current - 1
                                        } else {
                                            display_items.len().saturating_sub(1)
                                        };
                                        if let Some(DisplayItem::Album(_)) =
                                            display_items.get(wrapped_index)
                                        {
                                            // Find which album this corresponds to
                                            let mut album_count = 0;
                                            for (i, item) in display_items.iter().enumerate() {
                                                if matches!(item, DisplayItem::Album(_)) {
                                                    if i == wrapped_index {
                                                        self.album_list_state
                                                            .select(Some(album_count));
                                                        break;
                                                    }
                                                    album_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Artists mode, reset
                                self.panel_focus = PanelFocus::Artists;
                            }
                        }
                    }
                    MenuMode::Albums => {
                        match self.panel_focus {
                            PanelFocus::AlbumList => {
                                // Navigate all_albums list in Albums mode
                                if let Some(ref library) = self.library
                                    && !library.all_albums.is_empty()
                                {
                                    let current =
                                        self.all_albums_list_state.selected().unwrap_or(0);
                                    if current > 0 {
                                        self.all_albums_list_state.select(Some(current - 1));
                                    } else {
                                        // Wrap around to the bottom
                                        self.all_albums_list_state.select(Some(
                                            library.all_albums.len().saturating_sub(1),
                                        ));
                                    }
                                    // Reset track selection when navigating albums
                                    self.album_tracks_list_state.select(Some(0));
                                }
                            }
                            PanelFocus::AlbumTracks => {
                                // Navigate tracks in selected album
                                if let Some(ref library) = self.library
                                    && let Some(selected_album_index) =
                                        self.all_albums_list_state.selected()
                                    && let Some((_, album)) =
                                        library.all_albums.get(selected_album_index)
                                    && !album.tracks.is_empty()
                                {
                                    let current =
                                        self.album_tracks_list_state.selected().unwrap_or(0);
                                    if current > 0 {
                                        self.album_tracks_list_state.select(Some(current - 1));
                                    } else {
                                        // Wrap around to the bottom
                                        self.album_tracks_list_state
                                            .select(Some(album.tracks.len().saturating_sub(1)));
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Albums mode, reset
                                self.panel_focus = PanelFocus::AlbumList;
                            }
                        }
                    }
                }
            }
            MPDAction::NavigateDown => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        // Queue navigation is handled elsewhere
                    }
                    MenuMode::Artists => {
                        match self.panel_focus {
                            PanelFocus::Artists => {
                                // Navigate artists list
                                if let Some(ref library) = self.library
                                    && !library.artists.is_empty()
                                {
                                    let current = self.artist_list_state.selected().unwrap_or(0);
                                    let new_index =
                                        if current < library.artists.len().saturating_sub(1) {
                                            current + 1
                                        } else {
                                            // Wrap around to the top
                                            0
                                        };
                                    self.artist_list_state.select(Some(new_index));
                                    // Clear album selection when navigating artists
                                    self.album_list_state.select(None);
                                    self.album_display_list_state.select(None);

                                    // Lazy load the newly selected artist's albums
                                    if let Some(ref mut library) = self.library
                                        && let Err(e) = library.load_artist(client, new_index).await
                                    {
                                        log::warn!("Failed to load artist: {}", e);
                                    }
                                }
                            }
                            PanelFocus::Albums => {
                                // Navigate albums list using display list state
                                if let (Some(library), Some(selected_artist_index)) =
                                    (&self.library, self.artist_list_state.selected())
                                    && let Some(selected_artist) =
                                        library.get_artist(selected_artist_index)
                                {
                                    // Compute display list to get total count
                                    let (display_items, _album_indices) =
                                        compute_album_display_list(
                                            &selected_artist,
                                            &self.expanded_albums,
                                        );

                                    if !display_items.is_empty() {
                                        let current =
                                            self.album_display_list_state.selected().unwrap_or(0);
                                        if current < display_items.len().saturating_sub(1) {
                                            self.album_display_list_state.select(Some(current + 1));
                                        } else {
                                            // Wrap around to top
                                            self.album_display_list_state.select(Some(0));
                                        }

                                        // Update legacy album_list_state to point to current album if on album
                                        if let Some(DisplayItem::Album(_)) =
                                            display_items.get(current + 1)
                                        {
                                            // Find which album this corresponds to
                                            let mut album_count = 0;
                                            for (i, item) in display_items.iter().enumerate() {
                                                if matches!(item, DisplayItem::Album(_)) {
                                                    if i == current + 1 {
                                                        self.album_list_state
                                                            .select(Some(album_count));
                                                        break;
                                                    }
                                                    album_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Artists mode, reset
                                self.panel_focus = PanelFocus::Artists;
                            }
                        }
                    }
                    MenuMode::Albums => {
                        match self.panel_focus {
                            PanelFocus::AlbumList => {
                                // Navigate all_albums list in Albums mode
                                if let Some(ref library) = self.library
                                    && !library.all_albums.is_empty()
                                {
                                    let current =
                                        self.all_albums_list_state.selected().unwrap_or(0);
                                    if current < library.all_albums.len().saturating_sub(1) {
                                        self.all_albums_list_state.select(Some(current + 1));
                                    } else {
                                        // Wrap around to the top
                                        self.all_albums_list_state.select(Some(0));
                                    }
                                    // Reset track selection when navigating albums
                                    self.album_tracks_list_state.select(Some(0));
                                }
                            }
                            PanelFocus::AlbumTracks => {
                                // Navigate tracks in selected album
                                if let Some(ref library) = self.library
                                    && let Some(selected_album_index) =
                                        self.all_albums_list_state.selected()
                                    && let Some((_, album)) =
                                        library.all_albums.get(selected_album_index)
                                    && !album.tracks.is_empty()
                                {
                                    let current =
                                        self.album_tracks_list_state.selected().unwrap_or(0);
                                    if current < album.tracks.len().saturating_sub(1) {
                                        self.album_tracks_list_state.select(Some(current + 1));
                                    } else {
                                        // Wrap around to the top
                                        self.album_tracks_list_state.select(Some(0));
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Albums mode, reset
                                self.panel_focus = PanelFocus::AlbumList;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        // Mark library dirty for any panel navigation
        self.dirty.mark_library();
    }

    /// Handle scrolling by 15 items at a time
    async fn handle_scroll(&mut self, action: MPDAction, client: &Client) {
        match self.menu_mode {
            MenuMode::Queue => {
                if !self.queue.is_empty() {
                    let current = self.queue_list_state.selected().unwrap_or(0);
                    let new_index = match action {
                        MPDAction::ScrollUp => {
                            let potential = current.saturating_sub(15);
                            if potential == 0 && current == 0 {
                                // Already at top, wrap to bottom
                                self.queue.len().saturating_sub(1)
                            } else {
                                potential
                            }
                        }
                        MPDAction::ScrollDown => {
                            let potential =
                                std::cmp::min(current + 15, self.queue.len().saturating_sub(1));
                            if potential == self.queue.len().saturating_sub(1)
                                && current == self.queue.len().saturating_sub(1)
                            {
                                // Already at bottom, wrap to top
                                0
                            } else {
                                potential
                            }
                        }
                        _ => current,
                    };
                    self.queue_list_state.select(Some(new_index));
                    self.selected_queue_index = self.queue_list_state.selected();
                }
            }
            MenuMode::Artists => {
                // Handle scrolling based on current panel focus
                match self.panel_focus {
                    PanelFocus::Artists => {
                        if let Some(ref library) = self.library
                            && !library.artists.is_empty()
                        {
                            let current = self.artist_list_state.selected().unwrap_or(0);
                            let new_index = match action {
                                MPDAction::ScrollUp => {
                                    let potential = current.saturating_sub(15);
                                    if potential == 0 && current == 0 {
                                        // Already at top, wrap to bottom
                                        library.artists.len().saturating_sub(1)
                                    } else {
                                        potential
                                    }
                                }
                                MPDAction::ScrollDown => {
                                    let potential = std::cmp::min(
                                        current + 15,
                                        library.artists.len().saturating_sub(1),
                                    );
                                    if potential == library.artists.len().saturating_sub(1)
                                        && current == library.artists.len().saturating_sub(1)
                                    {
                                        // Already at bottom, wrap to top
                                        0
                                    } else {
                                        potential
                                    }
                                }
                                _ => current,
                            };
                            self.artist_list_state.select(Some(new_index));
                            // Clear album selection when scrolling artists
                            self.album_list_state.select(None);
                            self.album_display_list_state.select(None);

                            // Lazy load the newly selected artist's albums
                            if let Some(ref mut library) = self.library
                                && let Err(e) = library.load_artist(client, new_index).await
                            {
                                log::warn!("Failed to load artist: {}", e);
                            }
                        }
                    }
                    PanelFocus::Albums => {
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) = library.get_artist(selected_artist_index)
                        {
                            // Compute display list to get total count
                            let (display_items, _album_indices) =
                                compute_album_display_list(&selected_artist, &self.expanded_albums);
                            if !display_items.is_empty() {
                                let current = self.album_display_list_state.selected().unwrap_or(0);
                                let new_index = match action {
                                    MPDAction::ScrollUp => {
                                        let potential = current.saturating_sub(15);
                                        if potential == 0 && current == 0 {
                                            // Already at top, wrap to bottom
                                            display_items.len().saturating_sub(1)
                                        } else {
                                            potential
                                        }
                                    }
                                    MPDAction::ScrollDown => {
                                        let potential = std::cmp::min(
                                            current + 15,
                                            display_items.len().saturating_sub(1),
                                        );
                                        if potential == display_items.len().saturating_sub(1)
                                            && current == display_items.len().saturating_sub(1)
                                        {
                                            // Wrap around to top
                                            0
                                        } else {
                                            potential
                                        }
                                    }
                                    _ => current,
                                };
                                self.album_display_list_state.select(Some(new_index));

                                // Update the legacy album_list_state to point to the current album if on album
                                if let Some(DisplayItem::Album(_)) = display_items.get(new_index) {
                                    // Find which album this corresponds to
                                    let mut album_count = 0;
                                    for (i, item) in display_items.iter().enumerate() {
                                        if matches!(item, DisplayItem::Album(_)) {
                                            if i == new_index {
                                                self.album_list_state.select(Some(album_count));
                                                break;
                                            }
                                            album_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    PanelFocus::AlbumList | PanelFocus::AlbumTracks => {
                        // Not applicable in Artists mode
                    }
                }
            }
            MenuMode::Albums => {
                // Handle scrolling based on current panel focus in Albums mode
                match self.panel_focus {
                    PanelFocus::AlbumList => {
                        if let Some(ref library) = self.library
                            && !library.all_albums.is_empty()
                        {
                            let current = self.all_albums_list_state.selected().unwrap_or(0);
                            let new_index = match action {
                                MPDAction::ScrollUp => {
                                    let potential = current.saturating_sub(15);
                                    if potential == 0 && current == 0 {
                                        library.all_albums.len().saturating_sub(1)
                                    } else {
                                        potential
                                    }
                                }
                                MPDAction::ScrollDown => {
                                    let potential = std::cmp::min(
                                        current + 15,
                                        library.all_albums.len().saturating_sub(1),
                                    );
                                    if potential == library.all_albums.len().saturating_sub(1)
                                        && current == library.all_albums.len().saturating_sub(1)
                                    {
                                        0
                                    } else {
                                        potential
                                    }
                                }
                                _ => current,
                            };
                            self.all_albums_list_state.select(Some(new_index));
                            self.album_tracks_list_state.select(Some(0));
                        }
                    }
                    PanelFocus::AlbumTracks => {
                        if let Some(ref library) = self.library
                            && let Some(selected_album_index) =
                                self.all_albums_list_state.selected()
                            && let Some((_, album)) = library.all_albums.get(selected_album_index)
                            && !album.tracks.is_empty()
                        {
                            let current = self.album_tracks_list_state.selected().unwrap_or(0);
                            let new_index = match action {
                                MPDAction::ScrollUp => {
                                    let potential = current.saturating_sub(15);
                                    if potential == 0 && current == 0 {
                                        album.tracks.len().saturating_sub(1)
                                    } else {
                                        potential
                                    }
                                }
                                MPDAction::ScrollDown => {
                                    let potential = std::cmp::min(
                                        current + 15,
                                        album.tracks.len().saturating_sub(1),
                                    );
                                    if potential == album.tracks.len().saturating_sub(1)
                                        && current == album.tracks.len().saturating_sub(1)
                                    {
                                        0
                                    } else {
                                        potential
                                    }
                                }
                                _ => current,
                            };
                            self.album_tracks_list_state.select(Some(new_index));
                        }
                    }
                    PanelFocus::Artists | PanelFocus::Albums => {
                        // Not applicable in Albums mode
                    }
                }
            }
        }
        // Mark appropriate dirty flags for scrolling
        match self.menu_mode {
            MenuMode::Queue => self.dirty.mark_queue_selection(),
            MenuMode::Artists | MenuMode::Albums => self.dirty.mark_library(),
        }
    }

    /// Handle jumping to the top or bottom of the current list
    async fn handle_go_to_edge(&mut self, action: MPDAction, client: &Client) {
        match self.menu_mode {
            MenuMode::Queue => {
                if !self.queue.is_empty() {
                    let new_index = match action {
                        MPDAction::GoToTop => 0,
                        MPDAction::GoToBottom => self.queue.len().saturating_sub(1),
                        _ => return,
                    };
                    self.queue_list_state.select(Some(new_index));
                    self.selected_queue_index = self.queue_list_state.selected();
                }
            }
            MenuMode::Artists => {
                match self.panel_focus {
                    PanelFocus::Artists => {
                        if let Some(ref library) = self.library
                            && !library.artists.is_empty()
                        {
                            let new_index = match action {
                                MPDAction::GoToTop => 0,
                                MPDAction::GoToBottom => library.artists.len().saturating_sub(1),
                                _ => return,
                            };
                            self.artist_list_state.select(Some(new_index));
                            // Clear album selection when jumping in artists list
                            self.album_list_state.select(None);
                            self.album_display_list_state.select(None);

                            // Lazy load the newly selected artist's albums
                            if let Some(ref mut library) = self.library
                                && let Err(e) = library.load_artist(client, new_index).await
                            {
                                log::warn!("Failed to load artist: {}", e);
                            }
                        }
                    }
                    PanelFocus::Albums => {
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) = library.get_artist(selected_artist_index)
                        {
                            let (display_items, _album_indices) =
                                compute_album_display_list(&selected_artist, &self.expanded_albums);
                            if !display_items.is_empty() {
                                let new_index = match action {
                                    MPDAction::GoToTop => 0,
                                    MPDAction::GoToBottom => display_items.len().saturating_sub(1),
                                    _ => return,
                                };
                                self.album_display_list_state.select(Some(new_index));

                                // Update the legacy album_list_state if on an album
                                if let Some(DisplayItem::Album(_)) = display_items.get(new_index) {
                                    let mut album_count = 0;
                                    for (i, item) in display_items.iter().enumerate() {
                                        if matches!(item, DisplayItem::Album(_)) {
                                            if i == new_index {
                                                self.album_list_state.select(Some(album_count));
                                                break;
                                            }
                                            album_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    PanelFocus::AlbumList | PanelFocus::AlbumTracks => {
                        // Not applicable in Artists mode
                    }
                }
            }
            MenuMode::Albums => {
                match self.panel_focus {
                    PanelFocus::AlbumList => {
                        if let Some(ref library) = self.library
                            && !library.all_albums.is_empty()
                        {
                            let new_index = match action {
                                MPDAction::GoToTop => 0,
                                MPDAction::GoToBottom => library.all_albums.len().saturating_sub(1),
                                _ => return,
                            };
                            self.all_albums_list_state.select(Some(new_index));
                            self.album_tracks_list_state.select(Some(0));
                        }
                    }
                    PanelFocus::AlbumTracks => {
                        if let Some(ref library) = self.library
                            && let Some(selected_album_index) =
                                self.all_albums_list_state.selected()
                            && let Some((_, album)) = library.all_albums.get(selected_album_index)
                            && !album.tracks.is_empty()
                        {
                            let new_index = match action {
                                MPDAction::GoToTop => 0,
                                MPDAction::GoToBottom => album.tracks.len().saturating_sub(1),
                                _ => return,
                            };
                            self.album_tracks_list_state.select(Some(new_index));
                        }
                    }
                    PanelFocus::Artists | PanelFocus::Albums => {
                        // Not applicable in Albums mode
                    }
                }
            }
        }
        // Mark appropriate dirty flags for go to edge
        match self.menu_mode {
            MenuMode::Queue => self.dirty.mark_queue_selection(),
            MenuMode::Artists | MenuMode::Albums => self.dirty.mark_library(),
        }
    }

    /// Handle album expansion toggle
    async fn handle_album_toggle(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_artist_index)) =
            (&self.library, self.artist_list_state.selected())
            && let Some(selected_artist) = library.get_artist(selected_artist_index)
            && let Some(display_index) = self.album_display_list_state.selected()
        {
            let (display_items, _album_indices) =
                compute_album_display_list(&selected_artist, &self.expanded_albums);

            if let Some(display_item) = display_items.get(display_index) {
                match display_item {
                    DisplayItem::Album(album_name) => {
                        // Toggle album expansion
                        let album_key = (selected_artist.name.clone(), album_name.clone());

                        if self.expanded_albums.contains(&album_key) {
                            self.expanded_albums.remove(&album_key);
                        } else {
                            self.expanded_albums.insert(album_key);
                        }
                    }
                    DisplayItem::Song(_title, _duration, file_path) => {
                        // Add specific song to queue
                        let queue_was_empty = self.queue.is_empty();
                        if let Err(e) = client
                            .command(commands::Add::uri(file_path.to_str().unwrap()))
                            .await
                        {
                            error!("Error adding song to queue: {}", e);
                        } else if queue_was_empty {
                            // Start playback if queue was empty
                            if let Err(e) = client.command(commands::Play::current()).await {
                                error!("Error starting playback: {}", e);
                            }
                        }
                    }
                }
            }
        }
        // Mark library dirty for album expansion changes
        self.dirty.mark_library();
        Ok(())
    }

    /// Handle adding to queue in Artists mode - context-aware based on what's selected
    /// If on a song, add the song; if on an album, add the album
    async fn handle_add_to_queue_context_aware(
        &mut self,
        client: &Client,
    ) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_artist_index)) =
            (&self.library, self.artist_list_state.selected())
            && let Some(selected_artist) = library.get_artist(selected_artist_index)
            && let Some(display_index) = self.album_display_list_state.selected()
        {
            let (display_items, _album_indices) =
                compute_album_display_list(&selected_artist, &self.expanded_albums);

            if let Some(display_item) = display_items.get(display_index) {
                match display_item {
                    DisplayItem::Album(album_name) => {
                        // Add entire album to queue
                        if let Some(album) = selected_artist
                            .albums
                            .iter()
                            .find(|a| &a.name == album_name)
                        {
                            let queue_was_empty = self.queue.is_empty();
                            for song in &album.tracks {
                                if let Err(e) = client
                                    .command(commands::Add::uri(song.file_path.to_str().unwrap()))
                                    .await
                                {
                                    error!("Error adding song to queue: {}", e);
                                }
                            }
                            if queue_was_empty
                                && let Err(e) = client.command(commands::Play::current()).await
                            {
                                error!("Error starting playback: {}", e);
                            }
                        }
                    }
                    DisplayItem::Song(_title, _duration, file_path) => {
                        // Add specific song to queue
                        let queue_was_empty = self.queue.is_empty();
                        if let Err(e) = client
                            .command(commands::Add::uri(file_path.to_str().unwrap()))
                            .await
                        {
                            error!("Error adding song to queue: {}", e);
                        } else if queue_was_empty
                            && let Err(e) = client.command(commands::Play::current()).await
                        {
                            error!("Error starting playback: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle adding a specific song to queue in Albums mode (L key in songs pane)
    async fn handle_add_song_in_album_view(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_album_index)) =
            (&self.library, self.all_albums_list_state.selected())
            && let Some((_, album)) = library.all_albums.get(selected_album_index)
            && let Some(selected_track_index) = self.album_tracks_list_state.selected()
            && selected_track_index < album.tracks.len()
            && let Some(selected_song) = album.tracks.get(selected_track_index)
        {
            // Add the specific song to queue
            let queue_was_empty = self.queue.is_empty();
            if let Err(e) = client
                .command(commands::Add::uri(
                    selected_song.file_path.to_str().unwrap(),
                ))
                .await
            {
                error!("Error adding song to queue: {}", e);
            } else if queue_was_empty {
                // Start playback if queue was empty
                if let Err(e) = client.command(commands::Play::current()).await {
                    error!("Error starting playback: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Handle adding an entire album to queue in Albums mode (A/Enter key in albums pane)
    async fn handle_add_album_in_album_view(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_album_index)) =
            (&self.library, self.all_albums_list_state.selected())
            && let Some((_, album)) = library.all_albums.get(selected_album_index)
        {
            // Add all songs from the album to queue
            let queue_was_empty = self.queue.is_empty();
            for song in &album.tracks {
                if let Err(e) = client
                    .command(commands::Add::uri(song.file_path.to_str().unwrap()))
                    .await
                {
                    error!("Error adding song to queue: {}", e);
                }
            }
            // Start playback if queue was empty
            if queue_was_empty && let Err(e) = client.command(commands::Play::current()).await {
                error!("Error starting playback: {}", e);
            }
        }
        Ok(())
    }
}
