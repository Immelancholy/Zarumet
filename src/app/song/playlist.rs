use ratatui::widgets::ListState;

use crate::app::mpd::playlists::PlaylistSnapshot;
use crate::app::song::SongInfo;

#[derive(Debug)]
pub enum PlaylistDialog {
    Save {
        name: String,
        error: Option<String>,
    },
    Play {
        name: String,
    },
    Rename {
        original: String,
        name: String,
        error: Option<String>,
    },
    Delete {
        name: String,
        error: Option<String>,
    },
}

#[derive(Debug, Default)]
pub struct PlaylistBrowserState {
    pub names: Vec<String>,
    pub list_state: ListState,
    pub tracks: Vec<SongInfo>,
    pub track_state: ListState,
    pub tracks_focused: bool,
    pub selected_name: Option<String>,
    pub needs_refresh: bool,
    pub loading: bool,
    pub generation: u64,
    pub error: Option<String>,
    pub dialog: Option<PlaylistDialog>,
}

impl PlaylistBrowserState {
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.needs_refresh = true;
    }

    pub fn apply(&mut self, generation: u64, result: Result<PlaylistSnapshot, String>) {
        if generation != self.generation {
            return;
        }
        self.loading = false;
        match result {
            Ok(snapshot) => {
                let same_playlist = self.selected_name == snapshot.selected;
                // File + occurrence distinguishes duplicate entries while surviving inserted tracks.
                let previous = self.track_state.selected().and_then(|index| {
                    self.tracks.get(index).map(|song| {
                        let occurrence = self.tracks[..index]
                            .iter()
                            .filter(|track| track.file_path == song.file_path)
                            .count();
                        (song.file_path.clone(), occurrence)
                    })
                });
                self.names = snapshot.names;
                self.selected_name = snapshot.selected;
                self.list_state.select(
                    self.selected_name
                        .as_ref()
                        .and_then(|name| self.names.iter().position(|item| item == name)),
                );
                match snapshot.tracks {
                    Ok(tracks) => {
                        let index = if same_playlist {
                            previous.and_then(|(path, occurrence)| {
                                tracks
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, track)| track.file_path == path)
                                    .nth(occurrence)
                                    .map(|(index, _)| index)
                            })
                        } else {
                            None
                        };
                        self.track_state
                            .select(index.or_else(|| (!tracks.is_empty()).then_some(0)));
                        self.tracks = tracks;
                        self.error = None;
                    }
                    Err(error) => {
                        self.tracks.clear();
                        self.track_state.select(None);
                        self.error = Some(error);
                    }
                }
            }
            Err(error) => {
                self.tracks.clear();
                self.track_state.select(None);
                self.error = Some(error);
            }
        }
    }

    pub fn select(&mut self, index: usize) {
        let name = self.names.get(index).cloned();
        if self.selected_name != name {
            self.selected_name = name;
            self.list_state
                .select(self.selected_name.as_ref().map(|_| index));
            self.tracks.clear();
            self.track_state.select(None);
            self.error = None;
            self.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(names: &[&str], selected: &str) -> PlaylistSnapshot {
        PlaylistSnapshot {
            names: names.iter().map(|name| name.to_string()).collect(),
            selected: Some(selected.to_string()),
            tracks: Ok(Vec::new()),
        }
    }

    #[test]
    fn selection_follows_name_when_another_playlist_is_inserted() {
        let mut state = PlaylistBrowserState::default();
        state.apply(0, Ok(snapshot(&["Mix"], "Mix")));
        state.invalidate();
        state.apply(1, Ok(snapshot(&["Earlier", "Mix"], "Mix")));
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn stale_load_cannot_overwrite_a_newer_selection() {
        let mut state = PlaylistBrowserState::default();
        state.apply(0, Ok(snapshot(&["A", "B"], "A")));
        state.select(1);
        state.apply(0, Ok(snapshot(&["A", "B"], "A")));
        assert_eq!(state.selected_name.as_deref(), Some("B"));
        assert!(state.needs_refresh);
    }

    fn track(path: &str) -> SongInfo {
        SongInfo {
            title: path.to_string(),
            artist: String::new(),
            album: String::new(),
            file_path: path.into(),
            format: None,
            play_state: None,
            progress: None,
            elapsed: None,
            duration: None,
            disc_number: 0,
            track_number: 0,
            year: None,
            genre: None,
        }
    }

    #[test]
    fn track_selection_follows_the_same_duplicate_occurrence() {
        let mut state = PlaylistBrowserState::default();
        let mut first = snapshot(&["Mix"], "Mix");
        first.tracks = Ok(vec![track("a.flac"), track("a.flac")]);
        state.apply(0, Ok(first));
        state.track_state.select(Some(1));
        state.invalidate();
        let mut second = snapshot(&["Mix"], "Mix");
        second.tracks = Ok(vec![track("b.flac"), track("a.flac"), track("a.flac")]);
        state.apply(1, Ok(second));
        assert_eq!(state.track_state.selected(), Some(2));
    }

    #[test]
    fn failed_refresh_clears_tracks_instead_of_showing_stale_contents() {
        let mut state = PlaylistBrowserState::default();
        let mut first = snapshot(&["Mix"], "Mix");
        first.tracks = Ok(vec![track("a.flac")]);
        state.apply(0, Ok(first));
        state.invalidate();
        state.apply(1, Err("Unavailable".to_string()));
        assert!(state.tracks.is_empty());
        assert_eq!(state.track_state.selected(), None);
        assert_eq!(state.error.as_deref(), Some("Unavailable"));
    }
}
