use crate::App;
use crate::app::{
    MenuMode, PanelFocus,
    mpd_handler::MPDAction,
    ui::{DisplayItem, compute_album_display_list},
};
use mpd_client::Client;

impl App {
    /// Handle panel-specific navigation
    pub async fn handle_panel_navigation(&mut self, action: MPDAction, client: &Client) {
        match action {
            MPDAction::NavigateUp => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        // Queue navigation is handled elsewhere
                    }
                    MenuMode::Artists => {
                        match self.panel_focus {
                            PanelFocus::Artists => {
                                // Navigate artists list
                                if let Some(ref library) = self.library
                                    && !library.artists.is_empty()
                                {
                                    let current = self.artist_list_state.selected().unwrap_or(0);
                                    let new_index = if current > 0 {
                                        current - 1
                                    } else {
                                        // Wrap around to the bottom
                                        library.artists.len().saturating_sub(1)
                                    };
                                    self.artist_list_state.select(Some(new_index));
                                    // Clear album selection when navigating artists
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
                                // Navigate albums list using display list state
                                if let (Some(library), Some(selected_artist_index)) =
                                    (&self.library, self.artist_list_state.selected())
                                    && let Some(selected_artist) =
                                        library.get_artist(selected_artist_index)
                                {
                                    // Compute display list to get total count
                                    let (display_items, _album_indices) =
                                        compute_album_display_list(
                                            &selected_artist,
                                            &self.expanded_albums,
                                        );

                                    if !display_items.is_empty() {
                                        let current =
                                            self.album_display_list_state.selected().unwrap_or(0);
                                        if current > 0 {
                                            self.album_display_list_state.select(Some(current - 1));
                                        } else {
                                            // Wrap around to bottom
                                            self.album_display_list_state.select(Some(
                                                display_items.len().saturating_sub(1),
                                            ));
                                        }

                                        // Update the legacy album_list_state to point to the current album if on album
                                        let wrapped_index = if current > 0 {
                                            current - 1
                                        } else {
                                            display_items.len().saturating_sub(1)
                                        };
                                        if let Some(DisplayItem::Album(_)) =
                                            display_items.get(wrapped_index)
                                        {
                                            // Find which album this corresponds to
                                            let mut album_count = 0;
                                            for (i, item) in display_items.iter().enumerate() {
                                                if matches!(item, DisplayItem::Album(_)) {
                                                    if i == wrapped_index {
                                                        self.album_list_state
                                                            .select(Some(album_count));
                                                        break;
                                                    }
                                                    album_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Artists mode, reset
                                self.panel_focus = PanelFocus::Artists;
                            }
                        }
                    }
                    MenuMode::Albums => {
                        match self.panel_focus {
                            PanelFocus::AlbumList => {
                                // Navigate all_albums list in Albums mode
                                if let Some(ref library) = self.library
                                    && !library.all_albums.is_empty()
                                {
                                    let current =
                                        self.all_albums_list_state.selected().unwrap_or(0);
                                    if current > 0 {
                                        self.all_albums_list_state.select(Some(current - 1));
                                    } else {
                                        // Wrap around to the bottom
                                        self.all_albums_list_state.select(Some(
                                            library.all_albums.len().saturating_sub(1),
                                        ));
                                    }
                                    // Reset track selection when navigating albums
                                    self.album_tracks_list_state.select(Some(0));
                                }
                            }
                            PanelFocus::AlbumTracks => {
                                // Navigate tracks in selected album
                                if let Some(ref library) = self.library
                                    && let Some(selected_album_index) =
                                        self.all_albums_list_state.selected()
                                    && let Some((_, album)) =
                                        library.all_albums.get(selected_album_index)
                                    && !album.tracks.is_empty()
                                {
                                    let current =
                                        self.album_tracks_list_state.selected().unwrap_or(0);
                                    if current > 0 {
                                        self.album_tracks_list_state.select(Some(current - 1));
                                    } else {
                                        // Wrap around to the bottom
                                        self.album_tracks_list_state
                                            .select(Some(album.tracks.len().saturating_sub(1)));
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Albums mode, reset
                                self.panel_focus = PanelFocus::AlbumList;
                            }
                        }
                    }
                }
            }
            MPDAction::NavigateDown => {
                match self.menu_mode {
                    MenuMode::Queue => {
                        // Queue navigation is handled elsewhere
                    }
                    MenuMode::Artists => {
                        match self.panel_focus {
                            PanelFocus::Artists => {
                                // Navigate artists list
                                if let Some(ref library) = self.library
                                    && !library.artists.is_empty()
                                {
                                    let current = self.artist_list_state.selected().unwrap_or(0);
                                    let new_index =
                                        if current < library.artists.len().saturating_sub(1) {
                                            current + 1
                                        } else {
                                            // Wrap around to the top
                                            0
                                        };
                                    self.artist_list_state.select(Some(new_index));
                                    // Clear album selection when navigating artists
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
                                // Navigate albums list using display list state
                                if let (Some(library), Some(selected_artist_index)) =
                                    (&self.library, self.artist_list_state.selected())
                                    && let Some(selected_artist) =
                                        library.get_artist(selected_artist_index)
                                {
                                    // Compute display list to get total count
                                    let (display_items, _album_indices) =
                                        compute_album_display_list(
                                            &selected_artist,
                                            &self.expanded_albums,
                                        );

                                    if !display_items.is_empty() {
                                        let current =
                                            self.album_display_list_state.selected().unwrap_or(0);
                                        if current < display_items.len().saturating_sub(1) {
                                            self.album_display_list_state.select(Some(current + 1));
                                        } else {
                                            // Wrap around to top
                                            self.album_display_list_state.select(Some(0));
                                        }

                                        // Update legacy album_list_state to point to current album if on album
                                        if let Some(DisplayItem::Album(_)) =
                                            display_items.get(current + 1)
                                        {
                                            // Find which album this corresponds to
                                            let mut album_count = 0;
                                            for (i, item) in display_items.iter().enumerate() {
                                                if matches!(item, DisplayItem::Album(_)) {
                                                    if i == current + 1 {
                                                        self.album_list_state
                                                            .select(Some(album_count));
                                                        break;
                                                    }
                                                    album_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Artists mode, reset
                                self.panel_focus = PanelFocus::Artists;
                            }
                        }
                    }
                    MenuMode::Albums => {
                        match self.panel_focus {
                            PanelFocus::AlbumList => {
                                // Navigate all_albums list in Albums mode
                                if let Some(ref library) = self.library
                                    && !library.all_albums.is_empty()
                                {
                                    let current =
                                        self.all_albums_list_state.selected().unwrap_or(0);
                                    if current < library.all_albums.len().saturating_sub(1) {
                                        self.all_albums_list_state.select(Some(current + 1));
                                    } else {
                                        // Wrap around to the top
                                        self.all_albums_list_state.select(Some(0));
                                    }
                                    // Reset track selection when navigating albums
                                    self.album_tracks_list_state.select(Some(0));
                                }
                            }
                            PanelFocus::AlbumTracks => {
                                // Navigate tracks in selected album
                                if let Some(ref library) = self.library
                                    && let Some(selected_album_index) =
                                        self.all_albums_list_state.selected()
                                    && let Some((_, album)) =
                                        library.all_albums.get(selected_album_index)
                                    && !album.tracks.is_empty()
                                {
                                    let current =
                                        self.album_tracks_list_state.selected().unwrap_or(0);
                                    if current < album.tracks.len().saturating_sub(1) {
                                        self.album_tracks_list_state.select(Some(current + 1));
                                    } else {
                                        // Wrap around to the top
                                        self.album_tracks_list_state.select(Some(0));
                                    }
                                }
                            }
                            _ => {
                                // Invalid panel focus for Albums mode, reset
                                self.panel_focus = PanelFocus::AlbumList;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        // Mark library dirty for any panel navigation
        self.dirty.mark_library();
    }
}
