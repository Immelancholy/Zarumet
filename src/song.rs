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
        let album_artist = song
            .album_artists()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| artist.clone());
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
    pub async fn load_cover(&self, client: &Client) -> Option<Vec<u8>> {
        let uri = self.file_path.to_str()?;
        let art_data_result = client.album_art(&uri).await.ok()?;

        let (raw_data, _mime_type_option) = art_data_result?;

        return Some(raw_data.to_vec());
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
}

impl Library {
    pub async fn load_library(client: &Client) -> color_eyre::Result<Self> {
        let all_songs = client.command(commands::ListAllIn::root()).await?;

        let mut artists_map: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<SongInfo>>,
        > = std::collections::HashMap::new();

        for song in all_songs {
            let song_info = SongInfo::from_song(&song);
            let artist_name = song_info.album_artist.clone();
            let album_name = song_info.album.clone();

            let artist_entry = artists_map
                .entry(artist_name)
                .or_insert_with(std::collections::HashMap::new);
            let album_entry = artist_entry.entry(album_name).or_insert_with(Vec::new);
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

        Ok(Library { artists })
    }
}
