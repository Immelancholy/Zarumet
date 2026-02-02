use crate::App;
use log::error;
use mpd_client::{Client, commands};

impl App {
    /// Handle adding a specific song to queue in Albums mode (L key in songs pane)
    pub async fn handle_add_song_in_album_view(
        &mut self,
        client: &Client,
    ) -> color_eyre::Result<()> {
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
    pub async fn handle_add_album_in_album_view(
        &mut self,
        client: &Client,
    ) -> color_eyre::Result<()> {
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
