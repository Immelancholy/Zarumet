use crate::App;
use crate::app::ui::{DisplayItem, compute_album_display_list};
use log::error;
use mpd_client::{Client, commands};

impl App {
    /// Handle album expansion toggle
    pub async fn handle_album_toggle(&mut self, client: &Client) -> color_eyre::Result<()> {
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
}
