use crate::app::{
    SongInfo,
    ui::cache::cover_cache::{SharedCoverCache, get_prefetch_targets},
};
use mpd_client::Client;
use std::path::PathBuf;

use tokio::sync::mpsc;

/// Message type for cover art loading results
pub enum CoverArtMessage {
    Loaded(Option<Vec<u8>>, PathBuf),
}

/// Spawn a background task to load cover art with cache support
pub fn spawn_cover_art_loader(
    client: &Client,
    file_path: PathBuf,
    tx: mpsc::Sender<CoverArtMessage>,
    cache: SharedCoverCache,
) {
    let client = client.clone();
    let file_path_clone = file_path.clone();

    tokio::spawn(async move {
        // Check cache first
        {
            let mut cache_guard = cache.write().await;
            if let Some(cached) = cache_guard.get(&file_path_clone) {
                log::debug!("Cover art cache hit: {:?}", file_path_clone);
                let _ = tx
                    .send(CoverArtMessage::Loaded(
                        cached.data.clone(),
                        file_path_clone,
                    ))
                    .await;
                return;
            }

            // Check if already being fetched
            if cache_guard.is_pending(&file_path_clone) {
                log::debug!("Cover art already pending: {:?}", file_path_clone);
                return;
            }

            // Mark as pending
            cache_guard.mark_pending(file_path_clone.clone());
        }

        // Fetch from MPD
        let uri = file_path_clone.to_string_lossy();
        let result = client.album_art(&uri).await;

        let data = match result {
            Ok(Some((raw_data, _mime))) => Some(raw_data.to_vec()),
            Ok(None) => None,
            Err(e) => {
                log::debug!("Failed to load cover art: {}", e);
                None
            }
        };

        // Store in cache
        {
            let mut cache_guard = cache.write().await;
            cache_guard.insert(file_path_clone.clone(), data.clone());
        }

        // Send result back (ignore error if receiver dropped)
        let _ = tx
            .send(CoverArtMessage::Loaded(data, file_path_clone))
            .await;
    });
}

/// Spawn background tasks to prefetch cover art for adjacent queue items
pub fn spawn_prefetch_loaders(
    client: &Client,
    queue: &[SongInfo],
    current_index: Option<usize>,
    cache: SharedCoverCache,
) {
    let targets = get_prefetch_targets(queue, current_index);

    for file_path in targets {
        let client = client.clone();
        let cache = cache.clone();

        tokio::spawn(async move {
            // Check if already cached or pending
            {
                let mut cache_guard = cache.write().await;
                if cache_guard.contains(&file_path) || cache_guard.is_pending(&file_path) {
                    return;
                }
                cache_guard.mark_pending(file_path.clone());
            }

            // Fetch from MPD
            let uri = file_path.to_string_lossy();
            let result = client.album_art(&uri).await;

            let data = match result {
                Ok(Some((raw_data, _mime))) => Some(raw_data.to_vec()),
                Ok(None) => None,
                Err(e) => {
                    log::debug!("Failed to prefetch cover art: {}", e);
                    None
                }
            };

            // Store in cache (no need to send to channel - it's a prefetch)
            {
                let mut cache_guard = cache.write().await;
                cache_guard.insert(file_path.clone(), data);
                log::debug!("Prefetched cover art: {:?}", file_path);
            }
        });
    }
}
