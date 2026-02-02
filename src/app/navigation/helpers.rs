use crate::App;
use mpd_client::Client;

impl App {
    /// Preload all albums for Albums view and initialize selection
    pub async fn preload_albums_for_view(&mut self, client: &Client) {
        if let Some(ref mut library) = self.library
            && !library.all_albums_complete
        {
            log::info!("Preloading all albums for Albums view...");
            if let Err(e) = library.preload_all_albums(client).await {
                log::warn!("Failed to preload all albums: {}", e);
            }
        }

        if let Some(ref mut library) = self.library {
            library.ensure_albums_sorted();
        }

        if let Some(ref library) = self.library
            && !library.all_albums.is_empty()
            && self.all_albums_list_state.selected().is_none()
        {
            self.all_albums_list_state.select(Some(0));
        }
    }
}
