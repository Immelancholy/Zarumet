use ratatui::widgets::ListState;

use crate::app::mpd::directories::DirectoryEntry;

#[derive(Debug, Default)]
pub struct DirectoryPane {
    pub path: String,
    pub entries: Vec<DirectoryEntry>,
    pub list_state: ListState,
    pub loading: bool,
    pub error: Option<String>,
    pub needs_refresh: bool,
    pub generation: u64,
    selected_path: Option<String>,
}

impl DirectoryPane {
    pub fn selected(&self) -> Option<&DirectoryEntry> {
        if self.loading || self.needs_refresh || self.error.is_some() {
            return None;
        }
        self.list_state.selected().and_then(|i| self.entries.get(i))
    }

    pub fn invalidate(&mut self) {
        if let Some(entry) = self.selected() {
            self.selected_path = Some(entry.path.clone());
        }
        self.generation = self.generation.wrapping_add(1);
        self.needs_refresh = true;
    }

    pub fn enter(&mut self, path: String) {
        self.path = path;
        self.entries.clear();
        self.list_state = ListState::default();
        self.selected_path = None;
        self.error = None;
        self.invalidate();
    }

    pub fn parent(&mut self) {
        if self.path.is_empty() {
            return;
        }
        let child = self.path.clone();
        let parent = child
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent)
            .to_string();
        self.enter(parent);
        self.selected_path = Some(child);
    }

    pub fn apply(&mut self, generation: u64, result: Result<Vec<DirectoryEntry>, String>) {
        if generation != self.generation {
            return;
        }
        self.loading = false;
        match result {
            Ok(entries) => {
                let index = self
                    .selected_path
                    .as_ref()
                    .and_then(|path| entries.iter().position(|entry| &entry.path == path))
                    .or_else(|| {
                        (!entries.is_empty()).then(|| {
                            self.list_state
                                .selected()
                                .unwrap_or(0)
                                .min(entries.len() - 1)
                        })
                    });
                self.list_state.select(index);
                self.entries = entries;
                self.error = None;
                self.selected_path = None;
            }
            Err(error) => {
                self.entries.clear();
                self.list_state.select(None);
                self.error = Some(error);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct DirectoryBrowserState {
    pub current: DirectoryPane,
    pub preview: DirectoryPane,
    pub preview_focused: bool,
}

impl DirectoryBrowserState {
    pub fn focused(&mut self) -> &mut DirectoryPane {
        if self.preview_focused {
            &mut self.preview
        } else {
            &mut self.current
        }
    }

    pub fn sync_preview(&mut self) {
        if self.current.loading || self.current.needs_refresh {
            return;
        }
        let path = self
            .current
            .selected()
            .filter(|entry| entry.is_directory)
            .map(|entry| entry.path.clone());
        if path.as_deref() != Some(self.preview.path.as_str()) {
            let generation = self.preview.generation.wrapping_add(1);
            self.preview = DirectoryPane {
                generation,
                ..DirectoryPane::default()
            };
            if let Some(path) = path {
                self.preview.enter(path);
            } else {
                self.preview_focused = false;
            }
        }
    }

    pub fn invalidate(&mut self) {
        self.current.invalidate();
        self.preview.invalidate();
    }

    pub fn enter(&mut self, path: String) {
        self.current.enter(path);
        self.preview_focused = false;
        self.clear_preview();
    }

    fn clear_preview(&mut self) {
        let generation = self.preview.generation.wrapping_add(1);
        self.preview = DirectoryPane {
            generation,
            ..DirectoryPane::default()
        };
    }

    pub fn parent(&mut self) {
        if self.current.path.is_empty() {
            self.preview_focused = false;
            return;
        }
        self.current.parent();
        self.preview_focused = false;
        self.clear_preview();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(paths: &[&str]) -> Vec<DirectoryEntry> {
        paths
            .iter()
            .map(|path| DirectoryEntry {
                path: path.to_string(),
                is_directory: true,
            })
            .collect()
    }

    #[test]
    fn preview_tracks_left_selection_and_rejects_superseded_results() {
        let mut state = DirectoryBrowserState::default();
        state.current.apply(0, Ok(entries(&["A", "B"])));
        state.sync_preview();
        let old = state.preview.generation;
        state.current.list_state.select(Some(1));
        state.sync_preview();
        state.preview.loading = true;
        state.preview.apply(old, Ok(entries(&["A/old"])));
        assert_eq!(state.preview.path, "B");
        assert!(state.preview.entries.is_empty());
        assert!(state.preview.loading);
        state.current.entries[1].is_directory = false;
        state.preview_focused = true;
        state.sync_preview();
        assert!(state.preview.path.is_empty());
        assert!(!state.preview_focused);
    }

    #[test]
    fn refresh_keeps_both_selections_and_enter_clears_old_preview() {
        let mut state = DirectoryBrowserState::default();
        state.current.apply(0, Ok(entries(&["A", "B"])));
        state.current.list_state.select(Some(1));
        state.sync_preview();
        state.preview.needs_refresh = false;
        state
            .preview
            .apply(state.preview.generation, Ok(entries(&["B/one", "B/two"])));
        state.preview.list_state.select(Some(1));
        state.preview_focused = true;
        state.invalidate();
        state.sync_preview();
        assert!(state.focused().selected().is_none());
        state.current.needs_refresh = false;
        state
            .current
            .apply(state.current.generation, Ok(entries(&["0", "A", "B"])));
        state.sync_preview();
        state.preview.needs_refresh = false;
        state.preview.apply(
            state.preview.generation,
            Ok(entries(&["B/zero", "B/one", "B/two"])),
        );
        assert_eq!(state.focused().selected().unwrap().path, "B/two");
        let old = state.preview.generation;
        state.enter("B/two".into());
        state.preview.apply(old, Ok(entries(&["B/old"])));
        assert!(state.preview.entries.is_empty());
        assert!(!state.preview_focused);
        assert_eq!(state.current.path, "B/two");
    }

    #[test]
    fn parent_restores_child_selection_and_root_cannot_escape() {
        let mut state = DirectoryPane::default();
        state.enter("Nested/深い".to_string());
        state.parent();
        state.needs_refresh = false;
        state.apply(
            state.generation,
            Ok(entries(&["Nested/Other", "Nested/深い"])),
        );
        assert_eq!(state.path, "Nested");
        assert_eq!(state.list_state.selected(), Some(1));
        state.parent();
        let generation = state.generation;
        state.parent();
        assert_eq!(state.path, "");
        assert_eq!(state.generation, generation);
    }

    #[test]
    fn old_response_cannot_replace_new_directory_or_end_its_loading() {
        let mut state = DirectoryPane::default();
        state.enter("A".to_string());
        let old = state.generation;
        state.enter("B".to_string());
        state.loading = true;
        state.apply(old, Ok(entries(&["A/track"])));
        assert!(state.entries.is_empty());
        assert!(state.loading);
        assert_eq!(state.path, "B");
    }

    #[test]
    fn refresh_follows_path_and_removed_selection_uses_nearest_row() {
        let mut state = DirectoryPane::default();
        state.apply(0, Ok(entries(&["B", "C"])));
        state.list_state.select(Some(1));
        state.invalidate();
        assert!(state.selected().is_none());
        state.needs_refresh = false;
        state.apply(state.generation, Ok(entries(&["A", "B", "C"])));
        assert_eq!(state.selected().unwrap().path, "C");
        state.invalidate();
        state.needs_refresh = false;
        state.apply(state.generation, Ok(entries(&["A", "B"])));
        assert_eq!(state.selected().unwrap().path, "B");
    }

    #[test]
    fn error_and_empty_result_clear_stale_entries() {
        let mut state = DirectoryPane::default();
        state.apply(0, Ok(entries(&["A"])));
        state.apply(0, Err("Directory disappeared".to_string()));
        assert!(state.entries.is_empty());
        assert!(state.selected().is_none());
        state.apply(0, Ok(Vec::new()));
        assert!(state.error.is_none());
        assert!(state.selected().is_none());
    }
}
