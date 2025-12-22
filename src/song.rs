use mpd_client::{
    Client,
    client::CommandError,
    commands,
    commands::SetBinaryLimit,
    responses::{PlayState, Song},
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub has_explicit_album_artist: bool,
    pub album: String,
    pub file_path: PathBuf,
    pub format: Option<String>,
    pub play_state: Option<PlayState>,
    pub progress: Option<f64>,
    pub elapsed: Option<std::time::Duration>,
    pub duration: Option<std::time::Duration>,
    pub disc_number: u64,
    pub track_number: u64,
}

impl SongInfo {
    pub fn from_song(song: &Song) -> Self {
        let title = song
            .title()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Title".to_string());
        let artist = song
            .artists()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Artist".to_string());
        
        // Check if albumartist tag is explicitly set
        let explicit_album_artist = song.album_artists().first().map(|s| s.to_string());
        let has_explicit_album_artist = explicit_album_artist.is_some();
        let album_artist = explicit_album_artist.unwrap_or_else(|| artist.clone());
        
        let album = song
            .album()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Album".to_string());

        let file_path = song.file_path().to_path_buf();
        let format = song.format.clone();
        let duration = song.duration;
        let (disc_number, track_number) = song.number();

        Self {
            title,
            artist,
            album_artist,
            has_explicit_album_artist,
            album,
            file_path,
            format,
            play_state: None,
            progress: None,
            elapsed: None,
            duration,
            disc_number,
            track_number,
        }
    }
    pub async fn set_max_art_size(client: &Client, size_bytes: usize) -> Result<(), CommandError> {
        client.command(SetBinaryLimit(size_bytes)).await
    }

    /// Load album cover art for this song.
    /// Note: This is kept for potential future use, but the main loop now uses
    /// background loading via spawn_cover_art_loader for better responsiveness.
    #[allow(dead_code)]
    pub async fn load_cover(&self, client: &Client) -> Option<Vec<u8>> {
        let uri = self.file_path.to_str()?;
        let art_data_result = client.album_art(uri).await.ok()?;

        let (raw_data, _mime_type_option) = art_data_result?;

        Some(raw_data.to_vec())
    }

    pub fn update_playback_info(&mut self, play_state: Option<PlayState>, progress: Option<f64>) {
        self.play_state = play_state;
        self.progress = progress;
    }

    pub fn update_time_info(
        &mut self,
        elapsed: Option<std::time::Duration>,
        duration: Option<std::time::Duration>,
    ) {
        self.elapsed = elapsed;
        self.duration = duration;
    }

    /// Extract sample rate from the MPD format string.
    ///
    /// MPD returns format as "samplerate:bits:channels" (e.g., "44100:16:2").
    /// Returns None if format is not available or cannot be parsed.
    pub fn sample_rate(&self) -> Option<u32> {
        self.format
            .as_ref()
            .and_then(|f| f.split(':').next()?.parse().ok())
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub tracks: Vec<SongInfo>,
}

impl Album {
    /// Calculate the total duration of all tracks in the album
    pub fn total_duration(&self) -> Option<std::time::Duration> {
        let mut total_secs = 0u64;
        let mut has_duration = false;

        for track in &self.tracks {
            if let Some(duration) = track.duration {
                total_secs += duration.as_secs();
                has_duration = true;
            }
        }

        if has_duration {
            Some(std::time::Duration::from_secs(total_secs))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone)]
pub struct Library {
    pub artists: Vec<Artist>,
    /// Flattened list of all albums sorted alphabetically by album name.
    /// Each entry is (artist_name, Album).
    pub all_albums: Vec<(String, Album)>,
}

impl Library {
    pub async fn load_library(client: &Client) -> color_eyre::Result<Self> {
        let start_time = std::time::Instant::now();

        // Validate connection before loading
        Self::validate_connection(client).await?;

        // Try loading with retry logic
        let all_songs = Self::load_songs_with_retry(client).await?;

        let total_songs = all_songs.len();
        log::debug!("Loaded {} songs from MPD", total_songs);

        // PASS 1: Group all songs by album name to find the canonical album artist
        // This handles cases where some tracks in an album have explicit albumartist
        // tags while others fall back to the track artist.
        let mut albums_by_name: std::collections::HashMap<String, Vec<SongInfo>> =
            std::collections::HashMap::new();

        for song in &all_songs {
            let song_info = SongInfo::from_song(song);
            let album_name = song_info.album.clone();
            albums_by_name.entry(album_name).or_default().push(song_info);
        }

        // For each album, determine the canonical album artist:
        // - If any track has an explicit albumartist, use that
        // - Otherwise, use the most common artist among tracks
        let mut canonical_album_artist: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for (album_name, tracks) in &albums_by_name {
            // First, try to find any track with an explicit album artist
            let explicit_artist = tracks
                .iter()
                .find(|t| t.has_explicit_album_artist)
                .map(|t| t.album_artist.clone());

            let resolved_artist = if let Some(artist) = explicit_artist {
                artist
            } else {
                // Fall back to most common artist among tracks
                let mut artist_counts: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for track in tracks {
                    *artist_counts.entry(&track.album_artist).or_insert(0) += 1;
                }
                artist_counts
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(artist, _)| artist.to_string())
                    .unwrap_or_else(|| "Unknown Artist".to_string())
            };

            canonical_album_artist.insert(album_name.clone(), resolved_artist);
        }

        // PASS 2: Build the artists map using the canonical album artist
        let mut artists_map: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<SongInfo>>,
        > = std::collections::HashMap::new();

        for song in all_songs {
            let mut song_info = SongInfo::from_song(&song);
            let album_name = song_info.album.clone();

            // Use the canonical album artist for this album
            if let Some(canonical_artist) = canonical_album_artist.get(&album_name) {
                song_info.album_artist = canonical_artist.clone();
            }

            let artist_name = song_info.album_artist.clone();

            let artist_entry = artists_map.entry(artist_name).or_default();
            let album_entry = artist_entry.entry(album_name).or_default();
            album_entry.push(song_info);
        }

        let mut artists: Vec<Artist> = artists_map
            .into_iter()
            .map(|(artist_name, albums_map)| Artist {
                name: artist_name,
                albums: albums_map
                    .into_iter()
                    .map(|(album_name, tracks)| Album {
                        name: album_name,
                        tracks,
                    })
                    .collect(),
            })
            .collect();

        artists.sort_by(|a, b| a.name.cmp(&b.name));
        for artist in &mut artists {
            artist.albums.sort_by(|a, b| a.name.cmp(&b.name));
            for album in &mut artist.albums {
                album.tracks.sort_by(|a, b| {
                    a.disc_number
                        .cmp(&b.disc_number)
                        .then(a.track_number.cmp(&b.track_number))
                        .then(a.title.cmp(&b.title))
                });
            }
        }

        let total_artists = artists.len();
        let total_albums = artists.iter().map(|a| a.albums.len()).sum();
        let duration = start_time.elapsed();

        crate::logging::log_library_loading(
            total_songs,
            total_artists,
            total_albums,
            duration,
            true,
            None,
        );

        log::info!(
            "Library processing completed: {} artists, {} albums",
            total_artists,
            total_albums
        );

        // Build flattened all_albums list sorted alphabetically by album name
        let mut all_albums: Vec<(String, Album)> = Vec::new();
        for artist in &artists {
            for album in &artist.albums {
                all_albums.push((artist.name.clone(), album.clone()));
            }
        }
        // Sort alphabetically by album name (case-insensitive), then by artist name for stability
        all_albums.sort_by(|a, b| {
            a.1.name
                .to_lowercase()
                .cmp(&b.1.name.to_lowercase())
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });

        Ok(Library {
            artists,
            all_albums,
        })
    }

    /// Validate MPD connection with a simple ping
    async fn validate_connection(client: &Client) -> color_eyre::Result<()> {
        log::debug!("Validating MPD connection...");

        match client.command(commands::Status).await {
            Ok(_) => {
                log::debug!("MPD connection validated successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("MPD connection validation failed: {}", e);
                Err(color_eyre::eyre::eyre!(
                    "Failed to validate MPD connection: {}",
                    e
                ))
            }
        }
    }

    /// Load songs with retry logic and exponential backoff.
    /// Falls back to chunked loading if normal loading fails repeatedly.
    async fn load_songs_with_retry(client: &Client) -> color_eyre::Result<Vec<Song>> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 1000;

        for attempt in 1..=MAX_RETRIES {
            log::debug!("Loading MPD library (attempt {}/{})", attempt, MAX_RETRIES);

            match client.command(commands::ListAllIn::root()).await {
                Ok(songs) => {
                    log::debug!(
                        "Successfully loaded {} songs on attempt {}",
                        songs.len(),
                        attempt
                    );
                    return Ok(songs);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    log::warn!("Library loading attempt {} failed: {}", attempt, error_msg);

                    // Check if this is a protocol error
                    if error_msg.contains("protocol error") || error_msg.contains("invalid message")
                    {
                        log::error!(
                            "Protocol error detected on attempt {}: {}",
                            attempt,
                            error_msg
                        );
                    }

                    // If this is the last attempt, return error
                    if attempt == MAX_RETRIES {
                        let error = color_eyre::eyre::eyre!(
                            "Failed to load library after {} attempts: {}",
                            MAX_RETRIES,
                            error_msg
                        );
                        crate::logging::log_library_loading(
                            0,
                            0,
                            0,
                            std::time::Duration::from_secs(0),
                            false,
                            Some(&error_msg),
                        );
                        return Err(error);
                    }

                    // Exponential backoff: 1s, 2s, 4s
                    let delay_ms = BASE_DELAY_MS * 2_u64.pow(attempt - 1);
                    log::debug!("Waiting {}ms before retry...", delay_ms);
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }

        unreachable!()
    }
}
