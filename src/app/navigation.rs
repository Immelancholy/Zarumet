use log::error;
use mpd_client::{Client, commands};

use super::App;
use crate::app::mpd_handler::MPDAction;
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
                        }
                    }
                    MenuMode::Tracks => {
                        // Navigation is now handled by NavigateUp/Down actions based on panel focus
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
                        }
                    }
                    MenuMode::Tracks => {
                        // Navigation is now handled by NavigateUp/Down actions based on panel focus
                    }
                }
            }
            MPDAction::PlaySelected => {
                if let Some(selected) = self.queue_list_state.selected()
                    && selected < self.queue.len()
                {
                    // Play the song at the selected position in the queue
                    let song_position: mpd_client::commands::SongPosition = selected.into();
                    if let Err(e) = client
                        .command(mpd_client::commands::Play::song(song_position))
                        .await
                    {
                        error!("Error playing selected song: {}", e);
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
                    }
                }
            }
            MPDAction::Refresh => {
                // First, trigger MPD database update (equivalent to `mpc update`)
                log::info!("Updating MPD database...");
                match client.command(commands::Update::new()).await {
                    Ok(job_id) => {
                        log::info!("MPD database update started (job {})", job_id);
                        
                        // Wait for the update to complete by polling status
                        // The update is done when status.updating_db is None
                        let mut attempts = 0;
                        const MAX_ATTEMPTS: u32 = 300; // 30 seconds max wait (100ms * 300)
                        
                        loop {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            attempts += 1;
                            
                            match client.command(commands::Status).await {
                                Ok(status) => {
                                    if status.update_job.is_none() {
                                        log::info!("MPD database update completed");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to check update status: {}", e);
                                    break;
                                }
                            }
                            
                            if attempts >= MAX_ATTEMPTS {
                                log::warn!("MPD database update timed out, proceeding with library reload");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to start MPD database update: {}", e);
                        // Continue with library reload anyway
                    }
                }
                
                // Now reload the music library from MPD
                log::info!("Refreshing library...");
                match crate::song::Library::load_library(client).await {
                    Ok(new_library) => {
                        log::info!("Library refreshed successfully");
                        
                        // Preserve current artist selection if possible
                        let previous_artist_name = self.library.as_ref().and_then(|lib| {
                            self.artist_list_state.selected().and_then(|idx| {
                                lib.artists.get(idx).map(|a| a.name.clone())
                            })
                        });
                        
                        self.library = Some(new_library);
                        
                        // Try to restore artist selection by name
                        if let Some(prev_name) = previous_artist_name {
                            if let Some(ref library) = self.library {
                                if let Some(new_idx) = library.artists.iter().position(|a| a.name == prev_name) {
                                    self.artist_list_state.select(Some(new_idx));
                                } else if !library.artists.is_empty() {
                                    // Artist no longer exists, select first
                                    self.artist_list_state.select(Some(0));
                                }
                            }
                        } else if let Some(ref library) = self.library {
                            // No previous selection, select first if available
                            if !library.artists.is_empty() {
                                self.artist_list_state.select(Some(0));
                            }
                        }
                        
                        // Clear album selections since they may be stale
                        self.album_list_state.select(None);
                        self.album_display_list_state.select(None);
                        self.expanded_albums.clear();
                    }
                    Err(e) => {
                        error!("Failed to refresh library: {}", e);
                    }
                }
            }
            MPDAction::SwitchToQueueMenu => {
                self.menu_mode = MenuMode::Queue;
            }
            MPDAction::SwitchToTracks => {
                self.menu_mode = MenuMode::Tracks;
            }
            MPDAction::SwitchPanelLeft => {
                match self.panel_focus {
                    PanelFocus::Artists => {
                        // Already at artists panel, can't go left
                    }
                    PanelFocus::Albums => {
                        self.panel_focus = PanelFocus::Artists;
                        // Clear album selection when switching to artists panel
                        self.album_list_state.select(None);
                        self.album_display_list_state.select(None);
                    }
                }
            }
            MPDAction::SwitchPanelRight => {
                match self.panel_focus {
                    PanelFocus::Artists => {
                        self.panel_focus = PanelFocus::Albums;
                        // Initialize album selection when switching to albums panel
                        if let Some(ref library) = self.library
                            && let Some(selected_artist_index) = self.artist_list_state.selected()
                            && let Some(selected_artist) =
                                library.artists.get(selected_artist_index)
                        {
                            // Initialize display list state
                            self.album_display_list_state.select(Some(0));
                            if !selected_artist.albums.is_empty() {
                                self.album_list_state.select(Some(0));
                            }
                        }
                    }
                    PanelFocus::Albums => {
                        // Already at albums panel, can't go right
                    }
                }
            }
            MPDAction::NavigateUp | MPDAction::NavigateDown => {
                self.handle_panel_navigation(action).await;
            }
            MPDAction::GoToTop | MPDAction::GoToBottom => {
                self.handle_go_to_edge(action).await;
            }
            MPDAction::ToggleAlbumExpansion => {
                self.handle_album_toggle(client).await?;
            }
            MPDAction::AddSongToQueue => {
                self.handle_add_to_queue(client).await?;
            }
            MPDAction::CycleModeLeft => {
                // Cycle modes left: Queue -> Tracks -> Queue
                self.menu_mode = match self.menu_mode {
                    MenuMode::Queue => MenuMode::Tracks,
                    MenuMode::Tracks => MenuMode::Queue,
                };
            }
            MPDAction::CycleModeRight => {
                // Cycle modes right: Queue -> Tracks -> Queue
                self.menu_mode = match self.menu_mode {
                    MenuMode::Queue => MenuMode::Tracks,
                    MenuMode::Tracks => MenuMode::Queue,
                };
            }
            MPDAction::ScrollUp | MPDAction::ScrollDown => {
                self.handle_scroll(action).await;
            }
            _ => {
                // Execute MPD command for other actions, passing cached status
                if let Err(e) = action.execute(client, &self.config, self.mpd_status.as_ref()).await {
                    error!("Error executing MPD command: {}", e);
                }
            }
        }
        Ok(())
    }
}

impl App {
    /// Handle panel-specific navigation
    async fn handle_panel_navigation(&mut self, action: MPDAction) {
        match action {
            MPDAction::NavigateUp => {
                match self.panel_focus {
                    PanelFocus::Artists => {
                        // Navigate artists list
                        if let Some(ref library) = self.library
                            && !library.artists.is_empty()
                        {
                            let current = self.artist_list_state.selected().unwrap_or(0);
                            if current > 0 {
                                self.artist_list_state.select(Some(current - 1));
                            } else {
                                // Wrap around to the bottom
                                self.artist_list_state
                                    .select(Some(library.artists.len().saturating_sub(1)));
                            }
                            // Clear album selection when navigating artists
                            self.album_list_state.select(None);
                            self.album_display_list_state.select(None);
                        }
                    }
                    PanelFocus::Albums => {
                        // Navigate albums list using display list state
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) =
                                library.artists.get(selected_artist_index)
                        {
                            // Compute display list to get total count
                            let (display_items, _album_indices) =
                                compute_album_display_list(selected_artist, &self.expanded_albums);

                            if !display_items.is_empty() {
                                let current = self.album_display_list_state.selected().unwrap_or(0);
                                if current > 0 {
                                    self.album_display_list_state.select(Some(current - 1));
                                } else {
                                    // Wrap around to bottom
                                    self.album_display_list_state
                                        .select(Some(display_items.len().saturating_sub(1)));
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
                }
            }
            MPDAction::NavigateDown => {
                match self.panel_focus {
                    PanelFocus::Artists => {
                        // Navigate artists list
                        if let Some(ref library) = self.library
                            && !library.artists.is_empty()
                        {
                            let current = self.artist_list_state.selected().unwrap_or(0);
                            if current < library.artists.len().saturating_sub(1) {
                                self.artist_list_state.select(Some(current + 1));
                            } else {
                                // Wrap around to the top
                                self.artist_list_state.select(Some(0));
                            }
                            // Clear album selection when navigating artists
                            self.album_list_state.select(None);
                            self.album_display_list_state.select(None);
                        }
                    }
                    PanelFocus::Albums => {
                        // Navigate albums list using display list state
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) =
                                library.artists.get(selected_artist_index)
                        {
                            // Compute display list to get total count
                            let (display_items, _album_indices) =
                                compute_album_display_list(selected_artist, &self.expanded_albums);

                            if !display_items.is_empty() {
                                let current = self.album_display_list_state.selected().unwrap_or(0);
                                if current < display_items.len().saturating_sub(1) {
                                    self.album_display_list_state.select(Some(current + 1));
                                } else {
                                    // Wrap around to top
                                    self.album_display_list_state.select(Some(0));
                                }

                                // Update legacy album_list_state to point to current album if on album
                                if let Some(DisplayItem::Album(_)) = display_items.get(current + 1)
                                {
                                    // Find which album this corresponds to
                                    let mut album_count = 0;
                                    for (i, item) in display_items.iter().enumerate() {
                                        if matches!(item, DisplayItem::Album(_)) {
                                            if i == current + 1 {
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
                }
            }
            _ => {}
        }
    }

    /// Handle scrolling by 15 items at a time
    async fn handle_scroll(&mut self, action: MPDAction) {
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
            MenuMode::Tracks => {
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
                        }
                    }
                    PanelFocus::Albums => {
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) =
                                library.artists.get(selected_artist_index)
                        {
                            // Compute display list to get total count
                            let (display_items, _album_indices) =
                                compute_album_display_list(selected_artist, &self.expanded_albums);
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
                }
            }
        }
    }

    /// Handle jumping to the top or bottom of the current list
    async fn handle_go_to_edge(&mut self, action: MPDAction) {
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
            MenuMode::Tracks => {
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
                        }
                    }
                    PanelFocus::Albums => {
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) =
                                library.artists.get(selected_artist_index)
                        {
                            let (display_items, _album_indices) =
                                compute_album_display_list(selected_artist, &self.expanded_albums);
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
                }
            }
        }
    }

    /// Handle album expansion toggle
    async fn handle_album_toggle(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_artist_index)) =
            (&self.library, self.artist_list_state.selected())
            && let Some(selected_artist) = library.artists.get(selected_artist_index)
            && let Some(display_index) = self.album_display_list_state.selected()
        {
            let (display_items, _album_indices) =
                compute_album_display_list(selected_artist, &self.expanded_albums);

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
        Ok(())
    }

    /// Handle adding songs to queue
    async fn handle_add_to_queue(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_artist_index)) =
            (&self.library, self.artist_list_state.selected())
            && let Some(selected_artist) = library.artists.get(selected_artist_index)
            && let Some(selected_album_index) = self.album_list_state.selected()
            && let Some(selected_album) = selected_artist.albums.get(selected_album_index)
        {
            // Add all songs from the album to queue
            let queue_was_empty = self.queue.is_empty();
            for song in &selected_album.tracks {
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
