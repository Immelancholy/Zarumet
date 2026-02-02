use crate::App;
use crate::app::{
    MenuMode, PanelFocus,
    mpd_handler::MPDAction,
    ui::{DisplayItem, compute_album_display_list},
};
use mpd_client::Client;

impl App {
    /// Handle scrolling by 15 items at a time
    pub async fn handle_scroll(&mut self, action: MPDAction, client: &Client) {
        match self.menu_mode {
            MenuMode::Queue => {
                if !self.queue.is_empty() {
                    let current = self.queue_list_state.selected().unwrap_or(0);
                    let new_index = match action {
                        MPDAction::ScrollUp => {
                            let potential = current.saturating_sub(15);
                            if potential == 0 && current == 0 {
                                // Already at top, wrap to bottom
                                self.queue.len().saturating_sub(1)
                            } else {
                                potential
                            }
                        }
                        MPDAction::ScrollDown => {
                            let potential =
                                std::cmp::min(current + 15, self.queue.len().saturating_sub(1));
                            if potential == self.queue.len().saturating_sub(1)
                                && current == self.queue.len().saturating_sub(1)
                            {
                                // Already at bottom, wrap to top
                                0
                            } else {
                                potential
                            }
                        }
                        _ => current,
                    };
                    self.queue_list_state.select(Some(new_index));
                    self.selected_queue_index = self.queue_list_state.selected();
                }
            }
            MenuMode::Artists => {
                // Handle scrolling based on current panel focus
                match self.panel_focus {
                    PanelFocus::Artists => {
                        if let Some(ref library) = self.library
                            && !library.artists.is_empty()
                        {
                            let current = self.artist_list_state.selected().unwrap_or(0);
                            let new_index = match action {
                                MPDAction::ScrollUp => {
                                    let potential = current.saturating_sub(15);
                                    if potential == 0 && current == 0 {
                                        // Already at top, wrap to bottom
                                        library.artists.len().saturating_sub(1)
                                    } else {
                                        potential
                                    }
                                }
                                MPDAction::ScrollDown => {
                                    let potential = std::cmp::min(
                                        current + 15,
                                        library.artists.len().saturating_sub(1),
                                    );
                                    if potential == library.artists.len().saturating_sub(1)
                                        && current == library.artists.len().saturating_sub(1)
                                    {
                                        // Already at bottom, wrap to top
                                        0
                                    } else {
                                        potential
                                    }
                                }
                                _ => current,
                            };
                            self.artist_list_state.select(Some(new_index));
                            // Clear album selection when scrolling artists
                            self.album_list_state.select(None);
                            self.album_display_list_state.select(None);

                            // Lazy load the newly selected artist's albums
                            if let Some(ref mut library) = self.library
                                && let Err(e) = library.load_artist(client, new_index).await
                            {
                                log::warn!("Failed to load artist: {}", e);
                            }
                        }
                    }
                    PanelFocus::Albums => {
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) = library.get_artist(selected_artist_index)
                        {
                            // Compute display list to get total count
                            let (display_items, _album_indices) =
                                compute_album_display_list(&selected_artist, &self.expanded_albums);
                            if !display_items.is_empty() {
                                let current = self.album_display_list_state.selected().unwrap_or(0);
                                let new_index = match action {
                                    MPDAction::ScrollUp => {
                                        let potential = current.saturating_sub(15);
                                        if potential == 0 && current == 0 {
                                            // Already at top, wrap to bottom
                                            display_items.len().saturating_sub(1)
                                        } else {
                                            potential
                                        }
                                    }
                                    MPDAction::ScrollDown => {
                                        let potential = std::cmp::min(
                                            current + 15,
                                            display_items.len().saturating_sub(1),
                                        );
                                        if potential == display_items.len().saturating_sub(1)
                                            && current == display_items.len().saturating_sub(1)
                                        {
                                            // Wrap around to top
                                            0
                                        } else {
                                            potential
                                        }
                                    }
                                    _ => current,
                                };
                                self.album_display_list_state.select(Some(new_index));

                                // Update the legacy album_list_state to point to the current album if on album
                                if let Some(DisplayItem::Album(_)) = display_items.get(new_index) {
                                    // Find which album this corresponds to
                                    let mut album_count = 0;
                                    for (i, item) in display_items.iter().enumerate() {
                                        if matches!(item, DisplayItem::Album(_)) {
                                            if i == new_index {
                                                self.album_list_state.select(Some(album_count));
                                                break;
                                            }
                                            album_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    PanelFocus::AlbumList | PanelFocus::AlbumTracks => {
                        // Not applicable in Artists mode
                    }
                }
            }
            MenuMode::Albums => {
                // Handle scrolling based on current panel focus in Albums mode
                match self.panel_focus {
                    PanelFocus::AlbumList => {
                        if let Some(ref library) = self.library
                            && !library.all_albums.is_empty()
                        {
                            let current = self.all_albums_list_state.selected().unwrap_or(0);
                            let new_index = match action {
                                MPDAction::ScrollUp => {
                                    let potential = current.saturating_sub(15);
                                    if potential == 0 && current == 0 {
                                        library.all_albums.len().saturating_sub(1)
                                    } else {
                                        potential
                                    }
                                }
                                MPDAction::ScrollDown => {
                                    let potential = std::cmp::min(
                                        current + 15,
                                        library.all_albums.len().saturating_sub(1),
                                    );
                                    if potential == library.all_albums.len().saturating_sub(1)
                                        && current == library.all_albums.len().saturating_sub(1)
                                    {
                                        0
                                    } else {
                                        potential
                                    }
                                }
                                _ => current,
                            };
                            self.all_albums_list_state.select(Some(new_index));
                            self.album_tracks_list_state.select(Some(0));
                        }
                    }
                    PanelFocus::AlbumTracks => {
                        if let Some(ref library) = self.library
                            && let Some(selected_album_index) =
                                self.all_albums_list_state.selected()
                            && let Some((_, album)) = library.all_albums.get(selected_album_index)
                            && !album.tracks.is_empty()
                        {
                            let current = self.album_tracks_list_state.selected().unwrap_or(0);
                            let new_index = match action {
                                MPDAction::ScrollUp => {
                                    let potential = current.saturating_sub(15);
                                    if potential == 0 && current == 0 {
                                        album.tracks.len().saturating_sub(1)
                                    } else {
                                        potential
                                    }
                                }
                                MPDAction::ScrollDown => {
                                    let potential = std::cmp::min(
                                        current + 15,
                                        album.tracks.len().saturating_sub(1),
                                    );
                                    if potential == album.tracks.len().saturating_sub(1)
                                        && current == album.tracks.len().saturating_sub(1)
                                    {
                                        0
                                    } else {
                                        potential
                                    }
                                }
                                _ => current,
                            };
                            self.album_tracks_list_state.select(Some(new_index));
                        }
                    }
                    PanelFocus::Artists | PanelFocus::Albums => {
                        // Not applicable in Albums mode
                    }
                }
            }
        }
        // Mark appropriate dirty flags for scrolling
        match self.menu_mode {
            MenuMode::Queue => self.dirty.mark_queue_selection(),
            MenuMode::Artists | MenuMode::Albums => self.dirty.mark_library(),
        }
    }

    /// Handle jumping to the top or bottom of the current list
    pub async fn handle_go_to_edge(&mut self, action: MPDAction, client: &Client) {
        match self.menu_mode {
            MenuMode::Queue => {
                if !self.queue.is_empty() {
                    let new_index = match action {
                        MPDAction::GoToTop => 0,
                        MPDAction::GoToBottom => self.queue.len().saturating_sub(1),
                        _ => return,
                    };
                    self.queue_list_state.select(Some(new_index));
                    self.selected_queue_index = self.queue_list_state.selected();
                }
            }
            MenuMode::Artists => {
                match self.panel_focus {
                    PanelFocus::Artists => {
                        if let Some(ref library) = self.library
                            && !library.artists.is_empty()
                        {
                            let new_index = match action {
                                MPDAction::GoToTop => 0,
                                MPDAction::GoToBottom => library.artists.len().saturating_sub(1),
                                _ => return,
                            };
                            self.artist_list_state.select(Some(new_index));
                            // Clear album selection when jumping in artists list
                            self.album_list_state.select(None);
                            self.album_display_list_state.select(None);

                            // Lazy load the newly selected artist's albums
                            if let Some(ref mut library) = self.library
                                && let Err(e) = library.load_artist(client, new_index).await
                            {
                                log::warn!("Failed to load artist: {}", e);
                            }
                        }
                    }
                    PanelFocus::Albums => {
                        if let (Some(library), Some(selected_artist_index)) =
                            (&self.library, self.artist_list_state.selected())
                            && let Some(selected_artist) = library.get_artist(selected_artist_index)
                        {
                            let (display_items, _album_indices) =
                                compute_album_display_list(&selected_artist, &self.expanded_albums);
                            if !display_items.is_empty() {
                                let new_index = match action {
                                    MPDAction::GoToTop => 0,
                                    MPDAction::GoToBottom => display_items.len().saturating_sub(1),
                                    _ => return,
                                };
                                self.album_display_list_state.select(Some(new_index));

                                // Update the legacy album_list_state if on an album
                                if let Some(DisplayItem::Album(_)) = display_items.get(new_index) {
                                    let mut album_count = 0;
                                    for (i, item) in display_items.iter().enumerate() {
                                        if matches!(item, DisplayItem::Album(_)) {
                                            if i == new_index {
                                                self.album_list_state.select(Some(album_count));
                                                break;
                                            }
                                            album_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    PanelFocus::AlbumList | PanelFocus::AlbumTracks => {
                        // Not applicable in Artists mode
                    }
                }
            }
            MenuMode::Albums => {
                match self.panel_focus {
                    PanelFocus::AlbumList => {
                        if let Some(ref library) = self.library
                            && !library.all_albums.is_empty()
                        {
                            let new_index = match action {
                                MPDAction::GoToTop => 0,
                                MPDAction::GoToBottom => library.all_albums.len().saturating_sub(1),
                                _ => return,
                            };
                            self.all_albums_list_state.select(Some(new_index));
                            self.album_tracks_list_state.select(Some(0));
                        }
                    }
                    PanelFocus::AlbumTracks => {
                        if let Some(ref library) = self.library
                            && let Some(selected_album_index) =
                                self.all_albums_list_state.selected()
                            && let Some((_, album)) = library.all_albums.get(selected_album_index)
                            && !album.tracks.is_empty()
                        {
                            let new_index = match action {
                                MPDAction::GoToTop => 0,
                                MPDAction::GoToBottom => album.tracks.len().saturating_sub(1),
                                _ => return,
                            };
                            self.album_tracks_list_state.select(Some(new_index));
                        }
                    }
                    PanelFocus::Artists | PanelFocus::Albums => {
                        // Not applicable in Albums mode
                    }
                }
            }
        }
        // Mark appropriate dirty flags for go to edge
        match self.menu_mode {
            MenuMode::Queue => self.dirty.mark_queue_selection(),
            MenuMode::Artists | MenuMode::Albums => self.dirty.mark_library(),
        }
    }
}
