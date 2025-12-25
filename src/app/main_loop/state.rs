use mpd_client::Client;
use std::path::PathBuf;

use crate::app::Config;
use crate::app::PlayState;
use crate::app::SongInfo;
use crate::app::main_loop::{CoverArtMessage, spawn_cover_art_loader, spawn_prefetch_loaders};
use crate::app::ui::Protocol;
use crate::app::ui::cache::cover_cache::{SharedCoverCache, find_current_index};

#[cfg(target_os = "linux")]
use crate::app::audio::pipewire::{
    get_supported_rates, reset_sample_rate_async, set_sample_rate_async,
};
use crate::app::config::pipewire::resolve_bit_perfect_rate;

use tokio::sync::mpsc;

/// Check if the song changed and trigger cover art loading if needed
pub fn check_song_change(
    current_song_file: &mut Option<PathBuf>,
    current_song: &Option<SongInfo>,
    queue: &[SongInfo],
    client: &Client,
    cover_tx: &mpsc::Sender<CoverArtMessage>,
    protocol: &mut Protocol,
    cache: SharedCoverCache,
) {
    let new_song_file: Option<PathBuf> = current_song.as_ref().map(|song| song.file_path.clone());

    if new_song_file != *current_song_file {
        log::debug!(
            "Song changed: {:?} -> {:?}",
            current_song_file,
            new_song_file
        );

        // Clear protocol image when there's no current song
        if current_song.is_none() {
            protocol.image = None;
        }

        // Start loading cover art in background (uses cache internally)
        if let Some(ref file_path) = new_song_file {
            spawn_cover_art_loader(client, file_path.clone(), cover_tx.clone(), cache.clone());
        }

        // Prefetch adjacent queue items
        let current_idx = find_current_index(queue, current_song);
        spawn_prefetch_loaders(client, queue, current_idx, cache);

        *current_song_file = new_song_file;
    }
}

/// Handle PipeWire sample rate changes based on playback state and song changes
#[cfg(target_os = "linux")]
pub fn handle_pipewire_state_change(
    config: &Config,
    bit_perfect_enabled: bool,
    mpd_status: &Option<mpd_client::responses::Status>,
    current_song: &Option<SongInfo>,
    last_play_state: &mut Option<PlayState>,
    last_sample_rate: &mut Option<u32>,
) {
    if !bit_perfect_enabled || !config.pipewire.is_available() {
        return;
    }

    let current_play_state = mpd_status.as_ref().map(|s| s.state);
    let current_sample_rate = current_song.as_ref().and_then(|s| s.sample_rate());

    match current_play_state {
        Some(PlayState::Playing) => {
            // Check if we need to update sample rate:
            // 1. Just started playing (state changed)
            // 2. Song changed while playing (sample rate changed)
            let state_changed = current_play_state != *last_play_state;
            let rate_changed = current_sample_rate != *last_sample_rate;

            if (state_changed || rate_changed)
                && let Some(song_rate) = current_sample_rate
            {
                #[cfg(target_os = "linux")]
                if let Some(supported_rates) = get_supported_rates() {
                    let target_rate = resolve_bit_perfect_rate(song_rate, &supported_rates);
                    log::debug!(
                        "Setting PipeWire sample rate to {} (song rate: {})",
                        target_rate,
                        song_rate
                    );
                    // Fire-and-forget async call to avoid blocking the UI
                    tokio::spawn(async move {
                        let _ = set_sample_rate_async(target_rate).await;
                    });
                }
            }
        }
        Some(PlayState::Paused) | Some(PlayState::Stopped) | None => {
            // Paused or stopped - reset to automatic rate
            // Reset if we were playing, OR if last_play_state is None (unknown state after toggle)
            if *last_play_state == Some(PlayState::Playing) || last_play_state.is_none() {
                log::debug!(
                    "Resetting PipeWire sample rate (playback stopped, last_state={:?})",
                    last_play_state
                );
                // Fire-and-forget async call to avoid blocking the UI
                tokio::spawn(async {
                    let _ = reset_sample_rate_async().await;
                });
            }
        }
    }

    *last_play_state = current_play_state;
    *last_sample_rate = current_sample_rate;
}
