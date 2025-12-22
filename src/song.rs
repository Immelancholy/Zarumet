use mpd_client::{
    Client,
    client::CommandError,
    commands,
    commands::SetBinaryLimit,
    filter::{Filter, Operator},
    responses::{PlayState, Song},
    tag::Tag,
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

/// Lazy-loaded artist: initially only has the name, albums are loaded on demand
#[derive(Debug, Clone)]
pub struct LazyArtist {
    pub name: String,
    /// Albums for this artist - None means not yet loaded
    pub albums: Option<Vec<Album>>,
}

impl LazyArtist {
    /// Create a new lazy artist with just the name
    pub fn new(name: String) -> Self {
        Self { name, albums: None }
    }

    /// Check if this artist's albums have been loaded
    pub fn is_loaded(&self) -> bool {
        self.albums.is_some()
    }

    /// Get albums if loaded
    pub fn get_albums(&self) -> Option<&Vec<Album>> {
        self.albums.as_ref()
    }

    /// Convert to a regular Artist (returns empty albums if not loaded)
    pub fn to_artist(&self) -> Artist {
        Artist {
            name: self.name.clone(),
            albums: self.albums.clone().unwrap_or_default(),
        }
    }
}

/// Lazy-loading library that only fetches artist data when needed
#[derive(Debug, Clone)]
pub struct LazyLibrary {
    /// List of all artist names (loaded immediately)
    pub artists: Vec<LazyArtist>,
    /// Flattened list of all albums sorted alphabetically by album name.
    /// This is populated incrementally as artists are loaded.
    /// Each entry is (artist_name, Album).
    pub all_albums: Vec<(String, Album)>,
    /// Flag to track if all_albums is complete (all artists loaded)
    pub all_albums_complete: bool,
}

impl LazyLibrary {
    /// Initialize the library by loading just the artist names.
    /// This is fast because it only fetches tag values, not full song metadata.
    /// MPD command: list AlbumArtist
    pub async fn init(client: &Client) -> color_eyre::Result<Self> {
        let start_time = std::time::Instant::now();

        log::info!("Initializing lazy library (loading artist names only)...");

        // Get all unique album artists using the List command
        let album_artists_list = client
            .command(commands::List::new(Tag::AlbumArtist))
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to list album artists: {}", e))?;

        let mut artist_names: Vec<String> = album_artists_list
            .into_iter()
            .filter(|name| !name.is_empty())
            .collect();

        // Sort alphabetically
        artist_names.sort_by_key(|a| a.to_lowercase());

        let artists: Vec<LazyArtist> = artist_names.into_iter().map(LazyArtist::new).collect();

        let duration = start_time.elapsed();
        log::info!(
            "Lazy library initialized: {} artists in {:?}",
            artists.len(),
            duration
        );

        Ok(Self {
            artists,
            all_albums: Vec::new(),
            all_albums_complete: false,
        })
    }

    /// Load albums and songs for a specific artist by index.
    /// MPD command: find "(AlbumArtist == 'artist_name')" sort Album
    pub async fn load_artist(
        &mut self,
        client: &Client,
        artist_index: usize,
    ) -> color_eyre::Result<()> {
        if artist_index >= self.artists.len() {
            return Err(color_eyre::eyre::eyre!("Artist index out of bounds"));
        }

        // Skip if already loaded
        if self.artists[artist_index].is_loaded() {
            return Ok(());
        }

        let artist_name = self.artists[artist_index].name.clone();
        log::debug!("Loading albums for artist: {}", artist_name);

        let start_time = std::time::Instant::now();

        // Fetch all songs for this artist
        let filter = Filter::new(Tag::AlbumArtist, Operator::Equal, artist_name.clone());
        let find_cmd = commands::Find::new(filter).sort(Tag::Album);

        let songs = client
            .command(find_cmd)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to find songs for artist: {}", e))?;

        // Group songs by album
        let mut albums_map: std::collections::HashMap<String, Vec<SongInfo>> =
            std::collections::HashMap::new();

        for song in songs {
            let song_info = SongInfo::from_song(&song);
            let album_name = song_info.album.clone();
            albums_map.entry(album_name).or_default().push(song_info);
        }

        // Build album list
        let mut albums: Vec<Album> = albums_map
            .into_iter()
            .map(|(album_name, mut tracks)| {
                // Sort tracks by disc and track number
                tracks.sort_by(|a, b| {
                    a.disc_number
                        .cmp(&b.disc_number)
                        .then(a.track_number.cmp(&b.track_number))
                        .then(a.title.cmp(&b.title))
                });
                Album {
                    name: album_name,
                    tracks,
                }
            })
            .collect();

        // Sort albums alphabetically
        albums.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let duration = start_time.elapsed();
        log::debug!(
            "Loaded {} albums for '{}' in {:?}",
            albums.len(),
            artist_name,
            duration
        );

        // Update all_albums with newly loaded albums
        for album in &albums {
            // Check if this album is already in all_albums (avoid duplicates)
            let exists = self
                .all_albums
                .iter()
                .any(|(a_name, a)| a_name == &artist_name && a.name == album.name);
            if !exists {
                self.all_albums.push((artist_name.clone(), album.clone()));
            }
        }

        // Re-sort all_albums
        self.all_albums.sort_by(|a, b| {
            a.1.name
                .to_lowercase()
                .cmp(&b.1.name.to_lowercase())
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });

        // Store the loaded albums
        self.artists[artist_index].albums = Some(albums);

        // Check if all artists are now loaded
        self.all_albums_complete = self.artists.iter().all(|a| a.is_loaded());

        Ok(())
    }

    /// Get albums for an artist if loaded, None if not yet loaded
    pub fn get_artist_albums(&self, artist_index: usize) -> Option<&Vec<Album>> {
        self.artists.get(artist_index)?.get_albums()
    }

    /// Check if an artist's data has been loaded
    pub fn is_artist_loaded(&self, artist_index: usize) -> bool {
        self.artists
            .get(artist_index)
            .map(|a| a.is_loaded())
            .unwrap_or(false)
    }

    /// Get artist name by index
    pub fn get_artist_name(&self, artist_index: usize) -> Option<&str> {
        self.artists.get(artist_index).map(|a| a.name.as_str())
    }

    /// Get an Artist struct for the given index (for rendering).
    /// Returns None if index is out of bounds.
    /// If the artist's albums haven't been loaded, returns an Artist with empty albums.
    pub fn get_artist(&self, artist_index: usize) -> Option<Artist> {
        self.artists.get(artist_index).map(|a| a.to_artist())
    }

    /// Convert to a regular Library by loading all artists.
    /// This is useful for the Albums view which needs all albums.
    pub async fn to_full_library(&mut self, client: &Client) -> color_eyre::Result<Library> {
        log::info!("Converting lazy library to full library...");
        let start_time = std::time::Instant::now();

        // Load all artists that aren't already loaded
        for i in 0..self.artists.len() {
            if !self.artists[i].is_loaded() {
                self.load_artist(client, i).await?;
            }
        }

        // Build the full library
        let artists: Vec<Artist> = self
            .artists
            .iter()
            .map(|lazy_artist| Artist {
                name: lazy_artist.name.clone(),
                albums: lazy_artist.albums.clone().unwrap_or_default(),
            })
            .collect();

        let duration = start_time.elapsed();
        log::info!(
            "Full library loaded: {} artists, {} albums in {:?}",
            artists.len(),
            self.all_albums.len(),
            duration
        );

        Ok(Library {
            artists,
            all_albums: self.all_albums.clone(),
        })
    }

    /// Preload all albums for the Albums view.
    /// This loads all artists in parallel for faster loading.
    pub async fn preload_all_albums(&mut self, client: &Client) -> color_eyre::Result<()> {
        if self.all_albums_complete {
            return Ok(());
        }

        log::info!("Preloading all albums for Albums view...");
        let start_time = std::time::Instant::now();

        // Load all artists sequentially (could be parallelized in the future)
        for i in 0..self.artists.len() {
            if !self.artists[i].is_loaded() {
                self.load_artist(client, i).await?;
            }
        }

        let duration = start_time.elapsed();
        log::info!(
            "All albums preloaded: {} albums in {:?}",
            self.all_albums.len(),
            duration
        );

        Ok(())
    }
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
            albums_by_name
                .entry(album_name)
                .or_default()
                .push(song_info);
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

    /// Load the library using rmpc-style tag-based queries.
    /// This approach first lists all album artists, then fetches songs for each artist.
    /// Can be more efficient for incremental/lazy loading scenarios.
    #[allow(dead_code)]
    pub async fn load_library_by_artist(client: &Client) -> color_eyre::Result<Self> {
        let start_time = std::time::Instant::now();

        // Validate connection before loading
        Self::validate_connection(client).await?;

        log::debug!("Loading library using artist-based queries (rmpc-style)...");

        // Step 1: Get all unique album artists using the List command
        // MPD command: list AlbumArtist
        let album_artists_list = client
            .command(commands::List::new(Tag::AlbumArtist))
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to list album artists: {}", e))?;

        let album_artists: Vec<String> = album_artists_list.into_iter().collect();
        log::debug!("Found {} unique album artists", album_artists.len());

        // Step 2: For each album artist, fetch all their songs using Find command
        let mut artists: Vec<Artist> = Vec::new();
        let mut total_songs = 0usize;

        for artist_name in &album_artists {
            // Skip empty artist names
            if artist_name.is_empty() {
                continue;
            }

            // MPD command: find "(AlbumArtist == 'artist_name')"
            let filter = Filter::new(Tag::AlbumArtist, Operator::Equal, artist_name.clone());
            let find_cmd = commands::Find::new(filter).sort(Tag::Album);

            match client.command(find_cmd).await {
                Ok(songs) => {
                    if songs.is_empty() {
                        continue;
                    }

                    total_songs += songs.len();

                    // Group songs by album
                    let mut albums_map: std::collections::HashMap<String, Vec<SongInfo>> =
                        std::collections::HashMap::new();

                    for song in songs {
                        let song_info = SongInfo::from_song(&song);
                        let album_name = song_info.album.clone();
                        albums_map.entry(album_name).or_default().push(song_info);
                    }

                    // Build album list for this artist
                    let mut albums: Vec<Album> = albums_map
                        .into_iter()
                        .map(|(album_name, mut tracks)| {
                            // Sort tracks by disc and track number
                            tracks.sort_by(|a, b| {
                                a.disc_number
                                    .cmp(&b.disc_number)
                                    .then(a.track_number.cmp(&b.track_number))
                                    .then(a.title.cmp(&b.title))
                            });
                            Album {
                                name: album_name,
                                tracks,
                            }
                        })
                        .collect();

                    // Sort albums alphabetically
                    albums.sort_by(|a, b| a.name.cmp(&b.name));

                    artists.push(Artist {
                        name: artist_name.clone(),
                        albums,
                    });
                }
                Err(e) => {
                    log::warn!("Failed to fetch songs for artist '{}': {}", artist_name, e);
                    continue;
                }
            }
        }

        // Also fetch songs with no album artist tag (fallback to Artist tag)
        // MPD command: find "(AlbumArtist == '')"
        let empty_filter = Filter::new(Tag::AlbumArtist, Operator::Equal, "");
        let empty_find_cmd = commands::Find::new(empty_filter);

        if let Ok(orphan_songs) = client.command(empty_find_cmd).await
            && !orphan_songs.is_empty()
        {
            log::debug!(
                "Found {} songs without AlbumArtist tag, using Artist tag",
                orphan_songs.len()
            );
            total_songs += orphan_songs.len();

            // Group orphan songs by their Artist tag
            let mut orphan_artists_map: std::collections::HashMap<
                String,
                std::collections::HashMap<String, Vec<SongInfo>>,
            > = std::collections::HashMap::new();

            for song in orphan_songs {
                let song_info = SongInfo::from_song(&song);
                let artist_name = song_info.artist.clone();
                let album_name = song_info.album.clone();

                orphan_artists_map
                    .entry(artist_name)
                    .or_default()
                    .entry(album_name)
                    .or_default()
                    .push(song_info);
            }

            // Merge orphan artists into main artists list
            for (artist_name, albums_map) in orphan_artists_map {
                // Check if artist already exists
                if let Some(existing_artist) = artists.iter_mut().find(|a| a.name == artist_name) {
                    // Merge albums
                    for (album_name, mut tracks) in albums_map {
                        if let Some(existing_album) = existing_artist
                            .albums
                            .iter_mut()
                            .find(|a| a.name == album_name)
                        {
                            existing_album.tracks.append(&mut tracks);
                            existing_album.tracks.sort_by(|a, b| {
                                a.disc_number
                                    .cmp(&b.disc_number)
                                    .then(a.track_number.cmp(&b.track_number))
                                    .then(a.title.cmp(&b.title))
                            });
                        } else {
                            tracks.sort_by(|a, b| {
                                a.disc_number
                                    .cmp(&b.disc_number)
                                    .then(a.track_number.cmp(&b.track_number))
                                    .then(a.title.cmp(&b.title))
                            });
                            existing_artist.albums.push(Album {
                                name: album_name,
                                tracks,
                            });
                        }
                    }
                    existing_artist.albums.sort_by(|a, b| a.name.cmp(&b.name));
                } else {
                    // Create new artist
                    let mut albums: Vec<Album> = albums_map
                        .into_iter()
                        .map(|(album_name, mut tracks)| {
                            tracks.sort_by(|a, b| {
                                a.disc_number
                                    .cmp(&b.disc_number)
                                    .then(a.track_number.cmp(&b.track_number))
                                    .then(a.title.cmp(&b.title))
                            });
                            Album {
                                name: album_name,
                                tracks,
                            }
                        })
                        .collect();
                    albums.sort_by(|a, b| a.name.cmp(&b.name));
                    artists.push(Artist {
                        name: artist_name,
                        albums,
                    });
                }
            }
        }

        // Sort artists alphabetically
        artists.sort_by(|a, b| a.name.cmp(&b.name));

        let total_artists = artists.len();
        let total_albums: usize = artists.iter().map(|a| a.albums.len()).sum();
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
            "Library loaded (artist-based): {} songs, {} artists, {} albums in {:?}",
            total_songs,
            total_artists,
            total_albums,
            duration
        );

        // Build flattened all_albums list sorted alphabetically by album name
        let mut all_albums: Vec<(String, Album)> = Vec::new();
        for artist in &artists {
            for album in &artist.albums {
                all_albums.push((artist.name.clone(), album.clone()));
            }
        }
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

    /// Fetch songs for a specific artist using the Find command.
    /// This is the rmpc-style lazy loading approach.
    /// MPD command: find "(AlbumArtist == 'artist_name')"
    #[allow(dead_code)]
    pub async fn find_songs_by_artist(
        client: &Client,
        artist_name: &str,
    ) -> color_eyre::Result<Vec<Song>> {
        let filter = Filter::new(Tag::AlbumArtist, Operator::Equal, artist_name.to_string());
        let find_cmd = commands::Find::new(filter).sort(Tag::Album);

        client
            .command(find_cmd)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to find songs for artist: {}", e))
    }

    /// List all unique values for a tag (e.g., all artists, all albums).
    /// MPD command: list <tag>
    #[allow(dead_code)]
    pub async fn list_tag_values(client: &Client, tag: Tag) -> color_eyre::Result<Vec<String>> {
        let list_cmd = commands::List::new(tag);
        let result = client
            .command(list_cmd)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to list tag values: {}", e))?;

        Ok(result.into_iter().collect())
    }

    /// List albums for a specific artist using the List command with filter.
    /// MPD command: list Album "(AlbumArtist == 'artist_name')"
    #[allow(dead_code)]
    pub async fn list_albums_for_artist(
        client: &Client,
        artist_name: &str,
    ) -> color_eyre::Result<Vec<String>> {
        let filter = Filter::new(Tag::AlbumArtist, Operator::Equal, artist_name.to_string());
        let list_cmd = commands::List::new(Tag::Album).filter(filter);

        let result = client
            .command(list_cmd)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to list albums for artist: {}", e))?;

        Ok(result.into_iter().collect())
    }
}
