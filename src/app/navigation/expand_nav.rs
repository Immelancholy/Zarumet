use crate::App;
use crate::app::{
    PanelFocus,
    ui::{DisplayItem, compute_album_display_list, compute_albums_display_list_years},
};
use log::error;
use mpd_client::{Client, commands};
use ratatui::widgets::ListState;
use std::collections::HashSet;

// Generic album expansion toggle that works for any mode with expandable albums
async fn toggle_album_expansion_generic(
    queue: &mut [crate::app::SongInfo],
    display_list_state: &mut ListState,
    display_items: &[DisplayItem],
    album_indices: &[Option<usize>],
    expansion_set: &mut HashSet<(String, String)>,
    get_album_key: impl Fn(usize) -> Option<(String, String)>,
    client: &Client,
) -> color_eyre::Result<()> {
    if let Some(display_index) = display_list_state.selected()
        && let Some(display_item) = display_items.get(display_index)
    {
        match display_item {
            DisplayItem::Album(_) => {
                // Get the album index from the mapping
                if let Some(album_index) = album_indices.get(display_index).copied().flatten()
                    && let Some(album_key) = get_album_key(album_index)
                {
                    // Toggle expansion
                    if expansion_set.contains(&album_key) {
                        expansion_set.remove(&album_key);
                    } else {
                        expansion_set.insert(album_key);
                    }
                }
            }
            DisplayItem::Song(_title, _duration, file_path) => {
                // Add specific song to queue
                let queue_was_empty = queue.is_empty();
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
    Ok(())
}

impl App {
    /// Handle album expansion toggle for Artists mode  
    pub async fn handle_album_toggle(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_artist_index)) =
            (&self.library, self.artist_list_state.selected())
            && let Some(selected_artist) = library.get_artist(selected_artist_index)
        {
            let (display_items, album_indices) =
                compute_album_display_list(&selected_artist, &self.expanded_albums);

            // Prepare album lookup for Artists mode
            let selected_artist_clone = selected_artist.clone();
            let get_album_key = move |album_idx: usize| -> Option<(String, String)> {
                selected_artist_clone
                    .albums
                    .get(album_idx)
                    .map(|album| (selected_artist_clone.name.clone(), album.name.clone()))
            };

            // Use the generic helper
            toggle_album_expansion_generic(
                &mut self.queue,
                &mut self.album_display_list_state,
                &display_items,
                &album_indices,
                &mut self.expanded_albums,
                get_album_key,
                client,
            )
            .await?;
        }
        // Mark library dirty for album expansion changes
        self.dirty.mark_library();
        Ok(())
    }

    /// Handle album expansion toggle for Years mode
    pub async fn handle_year_album_toggle(&mut self, client: &Client) -> color_eyre::Result<()> {
        if let (Some(library), Some(selected_year_index)) =
            (&self.library, self.year_list_state.selected())
            && let Some(selected_year) = library.albums_by_year.get(selected_year_index)
        {
            let (display_items, album_indices) =
                compute_albums_display_list_years(&selected_year.1, &self.expanded_albums_years);

            // Prepare album lookup for Years mode
            let selected_year_clone = selected_year.clone();
            let get_album_key = move |album_idx: usize| -> Option<(String, String)> {
                selected_year_clone
                    .1
                    .get(album_idx)
                    .map(|(artist_name, album)| (artist_name.clone(), album.name.clone()))
            };

            // Use the generic helper
            toggle_album_expansion_generic(
                &mut self.queue,
                &mut self.year_albums_display_list_state,
                &display_items,
                &album_indices,
                &mut self.expanded_albums_years,
                get_album_key,
                client,
            )
            .await?;
        }
        // Mark library dirty for album expansion changes
        self.dirty.mark_library();
        Ok(())
    }

    /// Handle adding to queue in Artists mode - context-aware based on what's selected
    /// If on a song, add the song; if on an album, add the album
    pub async fn handle_add_to_queue_context_aware(
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

    /// Handle adding to queue in Years mode - context-aware based on panel focus and selection
    /// If on YearAlbums panel and on a song, add the song; if on YearAlbums panel and on an album, add the album
    pub async fn handle_add_to_queue_years_context_aware(
        &mut self,
        client: &Client,
    ) -> color_eyre::Result<()> {
        // Only handle when in YearAlbums panel
        if self.panel_focus != PanelFocus::YearAlbums {
            return Ok(());
        }

        if let (Some(library), Some(selected_year_index)) =
            (&self.library, self.year_list_state.selected())
            && let Some(selected_year) = library.albums_by_year.get(selected_year_index)
            && let Some(display_index) = self.year_albums_display_list_state.selected()
        {
            let (display_items, _album_indices) =
                compute_albums_display_list_years(&selected_year.1, &self.expanded_albums_years);

            if let Some(display_item) = display_items.get(display_index) {
                match display_item {
                    DisplayItem::Album(_album_name) => {
                        // Add entire album to queue
                        // Find which album this display item corresponds to
                        if let Some(album_idx) =
                            _album_indices.get(display_index).copied().flatten()
                            && let Some((_artist_name, album)) = selected_year.1.get(album_idx) {
                                let queue_was_empty = self.queue.is_empty();
                                for song in &album.tracks {
                                    if let Err(e) = client
                                        .command(commands::Add::uri(
                                            song.file_path.to_str().unwrap(),
                                        ))
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
}
