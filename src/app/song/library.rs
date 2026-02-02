use crate::app::{
    SongInfo,
    song::{Album, Artist, LazyArtist, artist::ArtistData},
};
use mpd_client::{
    client::Client,
    commands,
    filter::{Filter, Operator},
    tag::Tag,
};

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
        let album_artists_list = match client.command(commands::List::new(Tag::AlbumArtist)).await {
            Ok(list) => list,
            Err(e) => {
                log::error!("MPD List command failed for AlbumArtist tag: {}", e);
                log::error!("This usually indicates:");
                log::error!("  - MPD database corruption or inconsistency");
                log::error!("  - Permission issues with music directory");
                log::error!("  - Network/protocol issues with MPD server");
                log::error!("  - Missing or invalid AlbumArtist tags in music files");
                return Err(color_eyre::eyre::eyre!(
                    "Failed to list album artists: {}",
                    e
                ));
            }
        };

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

        // Skip if already loading - prevents concurrent loads
        if self.artists[artist_index].is_loading() {
            log::debug!(
                "Artist {} is already loading, skipping duplicate load",
                self.artists[artist_index].name
            );
            return Ok(());
        }

        // Set state to Loading to prevent concurrent loads
        let artist_name = self.artists[artist_index].name.clone();
        self.artists[artist_index].albums = ArtistData::Loading;
        log::debug!("Loading albums for artist: {}", artist_name);

        let start_time = std::time::Instant::now();

        // Fetch all songs for this artist
        let filter = Filter::new(Tag::AlbumArtist, Operator::Equal, artist_name.clone());
        let find_cmd = commands::Find::new(filter).sort(Tag::Album);

        let songs = match client.command(find_cmd).await {
            Ok(songs) => songs,
            Err(e) => {
                // Revert to NotLoaded on error
                self.artists[artist_index].albums = ArtistData::NotLoaded;
                log::error!("Failed to load songs for artist '{}': {}", artist_name, e);
                log::error!("This may indicate:");
                log::error!("  - Corrupted MPD database entries for this artist");
                log::error!("  - Missing or inaccessible music files");
                log::error!("  - Permission issues with music directory");
                log::error!("  - Network/protocol issues with MPD server");
                return Err(color_eyre::eyre::eyre!(
                    "Failed to find songs for artist '{}': {}",
                    artist_name,
                    e
                ));
            }
        };

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
                Album::new(album_name, tracks)
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
        self.artists[artist_index].albums = ArtistData::Loaded(albums);

        // Check if all artists are now loaded
        self.all_albums_complete = self.artists.iter().all(|a| a.is_loaded());

        Ok(())
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

        // Fetch ALL songs in the library at once using a filter that matches every song
        // This is faster than per-artist queries and more reliable than listallinfo.
        // We do this by requiring that the "file" tag exists:
        //   Filter::tag_exists(Tag::Other("file".into()))
        // (Conceptually similar to a raw MPD query like: find "(file != '')")
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
            // Skip if already loaded or currently loading to prevent concurrent access
            if artist.is_loaded() || artist.is_loading() {
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
                        Album::new(album_name, tracks)
                    })
                    .collect();

                albums.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                // Add to all_albums
                for album in &albums {
                    self.all_albums.push((artist.name.clone(), album.clone()));
                }

                artist.albums = ArtistData::Loaded(albums);
            } else {
                // Artist has no songs, mark as loaded with empty albums
                artist.albums = ArtistData::Loaded(Vec::new());
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
