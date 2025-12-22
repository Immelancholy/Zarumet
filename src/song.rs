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
    /// Flag to track if all_albums is sorted
    pub all_albums_sorted: bool,
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
            all_albums_sorted: false,
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

        // Mark as needing sort (defer sorting until all artists loaded or accessed)
        self.all_albums_sorted = false;

        // Store the loaded albums
        self.artists[artist_index].albums = Some(albums);

        // Check if all artists are now loaded
        self.all_albums_complete = self.artists.iter().all(|a| a.is_loaded());

        Ok(())
    }

    /// Check if an artist's data has been loaded
    pub fn is_artist_loaded(&self, artist_index: usize) -> bool {
        self.artists
            .get(artist_index)
            .map(|a| a.is_loaded())
            .unwrap_or(false)
    }

    /// Get an Artist struct for the given index (for rendering).
    /// Returns None if index is out of bounds.
    /// If the artist's albums haven't been loaded, returns an Artist with empty albums.
    pub fn get_artist(&self, artist_index: usize) -> Option<Artist> {
        self.artists.get(artist_index).map(|a| a.to_artist())
    }

    /// Ensure all_albums is sorted before access.
    /// This is a lazy sort: only sorts when needed.
    pub fn ensure_albums_sorted(&mut self) {
        if !self.all_albums_sorted {
            self.all_albums.sort_by(|a, b| {
                a.1.name
                    .to_lowercase()
                    .cmp(&b.1.name.to_lowercase())
                    .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
            });
            self.all_albums_sorted = true;
        }
    }

    /// Preload all albums for the Albums view.
    /// Uses a fast bulk approach: fetches all songs at once instead of per-artist.
    pub async fn preload_all_albums(&mut self, client: &Client) -> color_eyre::Result<()> {
        if self.all_albums_complete {
            return Ok(());
        }

        log::info!("Preloading all albums for Albums view (bulk)...");
        let start_time = std::time::Instant::now();

        // Fetch ALL songs in the library at once using find with a filter that matches everything
        // This is faster than per-artist queries and more reliable than listallinfo
        // MPD command: find "(file != '')"
        let filter = Filter::tag_exists(Tag::Other("file".into()));
        let all_songs = client
            .command(commands::Find::new(filter))
            .await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to find all songs: {}", e))?;

        // Group by artist -> album -> songs
        let mut artist_albums: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<SongInfo>>,
        > = std::collections::HashMap::new();

        for song in all_songs {
            let song_info = SongInfo::from_song(&song);
            // Use album artist for grouping (fall back to artist if not set)
            let artist_name = song
                .album_artists()
                .first()
                .map(|s| s.to_string())
                .unwrap_or_else(|| song_info.artist.clone());
            let album_name = song_info.album.clone();

            artist_albums
                .entry(artist_name)
                .or_default()
                .entry(album_name)
                .or_default()
                .push(song_info);
        }

        // Update each artist's albums
        for artist in &mut self.artists {
            if artist.is_loaded() {
                continue;
            }

            if let Some(albums_map) = artist_albums.remove(&artist.name) {
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

                albums.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                // Add to all_albums
                for album in &albums {
                    self.all_albums.push((artist.name.clone(), album.clone()));
                }

                artist.albums = Some(albums);
            } else {
                // Artist has no songs, mark as loaded with empty albums
                artist.albums = Some(Vec::new());
            }
        }

        // Sort all_albums once at the end
        self.all_albums.sort_by(|a, b| {
            a.1.name
                .to_lowercase()
                .cmp(&b.1.name.to_lowercase())
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });

        self.all_albums_sorted = true;
        self.all_albums_complete = true;

        let duration = start_time.elapsed();
        log::info!(
            "All albums preloaded: {} albums in {:?}",
            self.all_albums.len(),
            duration
        );

        Ok(())
    }
}
