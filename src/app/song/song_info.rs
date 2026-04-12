use mpd_client::{
    Client,
    client::CommandError,
    commands::SetBinaryLimit,
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
    pub year: Option<String>,
    pub genre: Option<String>,
}

impl SongInfo {
    pub fn sanitize_string(s: &str) -> String {
        let result: String = s
            .chars()
            .map(|c| match c {
                '\u{0000}'..='\u{001F}'
                | '\u{007F}'..='\u{009F}'
                | '\u{00AD}'
                | '\u{200B}'
                | '\u{200C}'
                | '\u{200D}'
                | '\u{2060}'
                | '\u{3164}'
                | '\u{FEFF}' => ' ',
                _ => c,
            })
            .collect();
        if result != s {
            log::debug!("Sanitized string: {:?} -> {:?}", s, result);
            for c in s.chars() {
                log::debug!("  Character U+{:04X}: '{}'", c as u32, c);
            }
        }
        result
    }

    pub fn from_song(song: &Song) -> Self {
        let file_path = song.file_path().to_path_buf();

        let title = song.title().map(Self::sanitize_string).unwrap_or_else(|| {
            // Use filename as title if no title in metadata
            file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(Self::sanitize_string)
                .unwrap_or_else(|| "Unknown Title".to_string())
        });

        let artist = song
            .artists()
            .first()
            .map(|s| Self::sanitize_string(s))
            .unwrap_or_else(|| "Unknown Artist".to_string());

        let album = song
            .album()
            .map(Self::sanitize_string)
            .unwrap_or_else(|| "Unknown Album".to_string());

        let format = song.format.clone();
        let duration = song.duration;
        let (disc_number, track_number) = song.number();
        let year = song
            .tags
            .get(&Tag::Date)
            .and_then(|dates| dates.first())
            .and_then(|date| {
                date.get(..4)
                    .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
                    .map(Self::sanitize_string)
            });
        let genre = song
            .tags
            .get(&Tag::Genre)
            .and_then(|genres| genres.first())
            .map(|s| Self::sanitize_string(s))
            .filter(|s| !s.is_empty());


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
            year,
            genre,
        }
    }
    pub async fn set_max_art_size(client: &Client, size_bytes: usize) -> Result<(), CommandError> {
        client.command(SetBinaryLimit(size_bytes)).await
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
