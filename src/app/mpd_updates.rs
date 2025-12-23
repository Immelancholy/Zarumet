use mpd_client::{Client, commands};

use super::App;
use crate::song::SongInfo;

/// Trait for MPD-related updates
pub trait MPDUpdates {
    /// Run full updates (status, queue, current song)
    async fn run_updates(&mut self, client: &Client) -> color_eyre::Result<()>;

    /// Run optimized updates based on what actually changed
    async fn run_optimized_updates(
        &mut self,
        client: &Client,
        needs_queue: bool,
        needs_current_song: bool,
    ) -> color_eyre::Result<()>;

    /// Update only status (lightweight, for player/mixer changes)
    async fn update_status_only(&mut self, client: &Client) -> color_eyre::Result<()>;
}

impl MPDUpdates for App {
    /// Run update functions concurrently with optimized result processing
    async fn run_updates(&mut self, client: &Client) -> color_eyre::Result<()> {
        // Full update - fetch everything
        self.run_optimized_updates(client, true, true).await
    }

    /// Run optimized updates - only fetch what we need
    async fn run_optimized_updates(
        &mut self,
        client: &Client,
        needs_queue: bool,
        needs_current_song: bool,
    ) -> color_eyre::Result<()> {
        // Always fetch status first to check versions
        let status = client.command(commands::Status).await?;

        // Check if queue actually changed (using playlist_version)
        let queue_changed = self
            .last_playlist_version
            .map(|v| v != status.playlist_version)
            .unwrap_or(true);

        // Check if current song changed (using songid)
        let current_song_id = status.current_song.map(|(_, id)| id);
        let song_changed = self.last_song_id != current_song_id;

        // Fetch queue only if needed AND it actually changed
        if needs_queue && queue_changed {
            log::debug!(
                "Queue changed: version {} -> {}",
                self.last_playlist_version.unwrap_or(0),
                status.playlist_version
            );
            let queue_songs = client.command(commands::Queue).await?;
            self.queue = queue_songs
                .into_iter()
                .map(|song_in_queue| SongInfo::from_song(&song_in_queue.song))
                .collect();

            // Update selected index to stay within bounds
            self.update_queue_selection();
            self.last_playlist_version = Some(status.playlist_version);

            // Mark queue as dirty for rendering
            self.dirty.mark_queue();
        }

        // Fetch current song only if needed AND it actually changed
        if needs_current_song && song_changed {
            log::debug!(
                "Song changed: {:?} -> {:?}",
                self.last_song_id,
                current_song_id
            );
            match client.command(commands::CurrentSong).await? {
                Some(song_in_queue) => {
                    self.current_song = Some(SongInfo::from_song(&song_in_queue.song));
                }
                None => {
                    self.current_song = None;
                }
            }
            self.last_song_id = current_song_id;

            // Mark current song and cover art as dirty
            self.dirty.mark_current_song();
            self.dirty.mark_cover_art();
        }

        // Always update playback info from status
        self.update_from_status(status);

        Ok(())
    }

    /// Lightweight status-only update (for progress bar, volume changes, etc.)
    async fn update_status_only(&mut self, client: &Client) -> color_eyre::Result<()> {
        let status = client.command(commands::Status).await?;
        self.update_from_status(status);
        Ok(())
    }
}

impl App {
    /// Update queue selection to stay within bounds
    fn update_queue_selection(&mut self) {
        match self.queue_list_state.selected() {
            Some(selected) => {
                if selected >= self.queue.len() {
                    if self.queue.is_empty() {
                        self.queue_list_state.select(None);
                    } else {
                        self.queue_list_state
                            .select(Some(self.queue.len().saturating_sub(1)));
                    }
                }
            }
            None => {
                if !self.queue.is_empty() {
                    self.queue_list_state.select(Some(0));
                }
            }
        }
        self.selected_queue_index = self.queue_list_state.selected();
    }

    /// Update app state from MPD status
    fn update_from_status(&mut self, status: mpd_client::responses::Status) {
        let progress = match (status.elapsed, status.duration) {
            (Some(elapsed), Some(duration)) => Some(elapsed.as_secs_f64() / duration.as_secs_f64()),
            _ => None,
        };

        if let Some(ref mut song) = self.current_song {
            song.update_playback_info(Some(status.state), progress);
            song.update_time_info(status.elapsed, status.duration);
        }

        // Mark status and progress as dirty
        self.dirty.mark_status();
        self.dirty.mark_progress();

        self.mpd_status = Some(status);
    }
}
