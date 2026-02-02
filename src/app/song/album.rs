use crate::app::SongInfo;

#[derive(Debug, Clone)]
pub struct Album {
    pub name: String,
    pub tracks: Vec<SongInfo>,
    /// Pre-computed total duration (computed once on construction)
    cached_total_duration: Option<std::time::Duration>,
}

impl Album {
    /// Create a new Album with pre-computed total duration
    pub fn new(name: String, tracks: Vec<SongInfo>) -> Self {
        let name = SongInfo::sanitize_string(&name);
        let cached_total_duration = Self::compute_total_duration(&tracks);
        Self {
            name,
            tracks,
            cached_total_duration,
        }
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
