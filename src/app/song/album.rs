use crate::app::SongInfo;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub tracks: Vec<SongInfo>,
    pub year: Option<String>,
    pub genre: Option<String>,
    /// Pre-computed total duration (computed once on construction)
    cached_total_duration: Option<std::time::Duration>,
}

impl Album {
    /// Create a new Album with pre-computed total duration
    pub fn new(name: String, tracks: Vec<SongInfo>) -> Self {
        let name = SongInfo::sanitize_string(&name);
        let cached_total_duration = Self::compute_total_duration(&tracks);
        let year = Self::compute_album_year(&tracks);
        let genre = Self::compute_album_genre(&tracks);
        Self {
            name,
            tracks,
            year,
            genre,
            cached_total_duration,
        }
    }

    fn compute_album_year(tracks: &[SongInfo]) -> Option<String> {
        tracks.iter().filter_map(|t| t.year.clone()).max()
    }

    fn compute_album_genre(tracks: &[SongInfo]) -> Option<String> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for genre in tracks.iter().filter_map(|t| t.genre.clone()) {
            counts.entry(genre).and_modify(|c| *c += 1).or_insert(1);
        }
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(genre, _)| genre)
    }

    /// Compute total duration from tracks (used during construction)
    fn compute_total_duration(tracks: &[SongInfo]) -> Option<std::time::Duration> {
        let mut total_secs = 0u64;
        let mut has_duration = false;

        for track in tracks {
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

    /// Get the total duration of all tracks in the album (cached)
    pub fn total_duration(&self) -> Option<std::time::Duration> {
        self.cached_total_duration
    }
}
