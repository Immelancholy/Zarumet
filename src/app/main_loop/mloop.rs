use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use mpd_client::client::{ConnectionEvent, Subsystem};
use mpd_client::responses::PlayState;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;

#[cfg(target_os = "linux")]
use crate::app::audio::pipewire;
use crate::app::config::pipewire::resolve_bit_perfect_rate;
use crate::app::main_loop::handle_pipewire_state_change;

use tokio::sync::mpsc;

use crate::App;
use crate::app::LazyLibrary;
use crate::app::main_loop::connect_to_mpd;

use crate::app::song::SongInfo;
use crate::app::ui::Protocol;
use crate::app::ui::WIDTH_CACHE;
use crate::app::ui::cache::cover_cache::{find_current_index, new_shared_cache};
use crate::app::ui::rendering::render;
use crate::app::{
    MessageType, StatusMessage, event_handlers::EventHandlers, mpd_updates::MPDUpdates,
};

use crate::app::main_loop::check_song_change;

use crate::app::main_loop::{CoverArtMessage, spawn_cover_art_loader, spawn_prefetch_loaders};

/// Interval for progress bar updates when playing (in milliseconds)
const PROGRESS_UPDATE_INTERVAL_MS: u64 = 500;

/// Trait for main application loop
pub trait AppMainLoop {
    async fn run(self, terminal: DefaultTerminal) -> color_eyre::Result<()>
    where
        Self: Sized;
}

/// Connect to MPD via Unix socket or TCP based on address format

impl AppMainLoop for App {
    /// Run the application's main loop.
    async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;

        // Connect to MPD
        log::info!(
            "Attempting to connect to MPD at: {}",
            self.config.mpd.address
        );

        let (client, mut state_changes) = connect_to_mpd(&self.config.mpd.address)
            .await
            .inspect_err(|e| {
                crate::logging::log_mpd_connection(
                    &self.config.mpd.address,
                    false,
                    Some(&e.to_string()),
                );
            })?;

        crate::logging::log_mpd_connection(&self.config.mpd.address, true, None);

        match SongInfo::set_max_art_size(&client, 5 * 1024 * 1024).await {
            Ok(_) => {
                log::debug!("Set MPD binary limit to 5MB");
            }
            Err(e) => {
                log::warn!("Failed to set MPD binary limit: {}", e);
            }
        }

        // Load library (lazy - only artist names initially)
        match LazyLibrary::init(&client).await {
            Ok(library) => {
                self.library = Some(library);

                // Initialize artist selection if library has artists
                if let Some(ref library) = self.library
                    && !library.artists.is_empty()
                {
                    self.artist_list_state.select(Some(0));

                    // Load the first artist's albums immediately for better UX
                    if let Some(ref mut lib) = self.library
                        && let Err(e) = lib.load_artist(&client, 0).await
                    {
                        log::warn!("Failed to load first artist: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to initialize music library: {}", e);
                log::error!(
                    "This may be due to MPD protocol issues, corrupted database, or permission problems."
                );
                log::error!(
                    "Zarumet will continue running but library features will be unavailable."
                );
                self.library = None;
            }
        }

        // Set up the image picker and protocol
        let mut picker = Picker::from_query_stdio().unwrap();
        picker.set_background_color([0, 0, 0, 0]);

        // Fetch initial song info and status
        self.run_updates(&client).await?;

        // Track the current song's file path
        let mut current_song_file: Option<PathBuf> = self
            .current_song
            .as_ref()
            .map(|song| song.file_path.clone());

        // Update samplerate on startup if needed
        #[cfg(target_os = "linux")]
        {
            let initial_play_state = self.mpd_status.as_ref().map(|s| s.state);
            let initial_sample_rate = self.current_song.as_ref().and_then(|s| s.sample_rate());

            if self.bit_perfect_enabled
                && self.config.pipewire.is_available()
                && let (Some(PlayState::Playing), Some(song_rate)) =
                    (initial_play_state, initial_sample_rate)
                && let Some(supported_rates) = pipewire::get_supported_rates()
            {
                let target_rate = resolve_bit_perfect_rate(song_rate, &supported_rates);
                log::debug!(
                    "Setting PipeWire sample rate to {} on startup (song rate: {})",
                    target_rate,
                    song_rate
                );
                let _ = pipewire::set_sample_rate_async(target_rate).await;
            }
        }

        // Channel for cover art loading results
        let (cover_tx, mut cover_rx) = mpsc::channel::<CoverArtMessage>(1);

        // Create shared cover art cache
        let cover_cache = new_shared_cache();

        // Load initial cover art in background
        if let Some(ref song) = self.current_song {
            let file_path = song.file_path.clone();
            spawn_cover_art_loader(&client, file_path, cover_tx.clone(), cover_cache.clone());
        }

        // Prefetch cover art for adjacent queue items
        let current_idx = find_current_index(&self.queue, &self.current_song);
        spawn_prefetch_loaders(&client, &self.queue, current_idx, cover_cache.clone());

        // Create protocol with no initial image (will be loaded async)
        let mut protocol = Protocol { image: None };

        // Progress update interval
        let progress_interval =
            tokio::time::interval(Duration::from_millis(PROGRESS_UPDATE_INTERVAL_MS));
        tokio::pin!(progress_interval);

        // Set up signal handlers for graceful shutdown (Unix only)
        #[cfg(unix)]
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to set up SIGINT handler");
        #[cfg(unix)]
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to set up SIGTERM handler");

        log::info!("Entering event-driven main loop");

        while self.running {
            // Check terminal size for dirty tracking
            let term_size = terminal.size()?;
            self.dirty
                .check_terminal_size(term_size.width, term_size.height);

            // Only render if something has changed
            if self.dirty.any_dirty() {
                terminal.draw(|frame| {
                    render(
                        frame,
                        &mut protocol,
                        &self.current_song,
                        &self.queue,
                        &mut self.queue_list_state,
                        &self.config,
                        &self.menu_mode,
                        &self.library,
                        &mut self.artist_list_state,
                        &mut self.album_list_state,
                        &mut self.album_display_list_state,
                        &mut self.all_albums_list_state,
                        &mut self.album_tracks_list_state,
                        &self.panel_focus,
                        &self.expanded_albums,
                        &self.mpd_status,
                        &self.key_binds,
                        self.bit_perfect_enabled,
                        self.show_config_warnings_popup,
                        &self.config_warnings,
                        &self.status_message,
                    )
                })?;

                if let Some(ref mut img) = protocol.image {
                    img.last_encoding_result();
                }

                // Clear dirty flags after render
                self.dirty.clear_all();
            }

            // Update key bindings for timeouts and mark dirty if state changed
            let was_awaiting = self.key_binds.is_awaiting_input();
            self.key_binds.update();
            if was_awaiting && !self.key_binds.is_awaiting_input() {
                // Timeout occurred - need to clear the sequence indicator
                self.dirty.mark_key_sequence();
            }

            self.check_status_message_expiry();
            self.check_animation_updates();

            // Log width cache statistics periodically
            static CACHE_LOG_COUNTER: AtomicU64 = AtomicU64::new(0);
            let counter = CACHE_LOG_COUNTER.fetch_add(1, Ordering::Relaxed);

            // Log every ~600 iterations (about every 30 seconds at 20 FPS)
            if counter.is_multiple_of(600) && counter > 0 {
                WIDTH_CACHE.with(|cache| {
                    let cache = cache.borrow();
                    if cache.total_accesses() > 100 {
                        // Only log if there's meaningful activity
                        cache.log_stats();
                    }
                });

                // Log cover art cache stats
                let cache_guard = cover_cache.read().await;
                cache_guard.log_stats();
            }

            // Event-driven loop using tokio::select!
            tokio::select! {
                // Keyboard events (with short timeout for responsive UI)
                _ = tokio::time::sleep(Duration::from_millis(10)) => {
                    // Check for keyboard events non-blocking
                    if crossterm::event::poll(Duration::from_millis(0))? {
                        self.handle_crossterm_events(&client).await?;

                        // If user action requires update, do it immediately
                        if self.force_update {
                            self.run_updates(&client).await?;
                            self.force_update = false;

                            // Check for song change after update
                            check_song_change(
                                &mut current_song_file,
                                &self.current_song,
                                &self.queue,
                                &client,
                                &cover_tx,
                                &mut protocol,
                                cover_cache.clone(),
                            );
                        }
                    }
                }

                // MPD state change notifications
                mpd_event = state_changes.next() => {
                    match mpd_event {
                        Some(ConnectionEvent::SubsystemChange(subsystem)) => {
                            log::debug!("MPD subsystem change: {:?}", subsystem);

                            // Optimize updates based on which subsystem changed
                            match subsystem {
                                // Player state changes (play/pause/stop/seek) - need status + maybe current song
                                Subsystem::Player => {
                                    self.run_optimized_updates(&client, false, true).await?;
                                }
                                // Mixer changes (volume) - only need status
                                Subsystem::Mixer => {
                                    self.update_status_only(&client).await?;
                                }
                                // Options changes (repeat, random, etc.) - only need status
                                Subsystem::Options => {
                                    self.update_status_only(&client).await?;
                                }
                                // Queue/playlist changes - need full update
                                Subsystem::Queue => {
                                    self.run_updates(&client).await?;
                                }
                                // Stored playlist changes - may affect queue if current playlist modified
                                Subsystem::StoredPlaylist => {
                                    self.run_updates(&client).await?;
                                }
                                Subsystem::Update => {
                                    if self.library_reload_pending {
                                        let was_user_initiated = self.user_initiated_reload;
                                        self.update_in_progress = false;  // Allow new refreshes
                                        self.user_initiated_reload = false;  // Clear this flag

                                        // Show success message only if user initiated the update
                                        if was_user_initiated {  // ← Changed from self.update_in_progress
                                            self.set_status_message(StatusMessage {
                                                text: String::new(),
                                                created_at: std::time::Instant::now(),
                                                message_type: MessageType::Success,
                                            });
                                        }
                                        log::info!("Database update completed, reloading library...");

                                        // Now reload the music library from MPD
                                        log::info!("Refreshing library...");
                                        match LazyLibrary::init(&client).await {
                                            Ok(new_library) => {
                                                log::info!("Library refreshed successfully");

                                                self.library = Some(new_library);

                                                // Try to restore artist selection by name
                                                if let Some(prev_name) = self.pending_artist_index.take() {
                                                    if let Some(ref mut library) = self.library {
                                                        if let Some(new_idx) =
                                                            library.artists.iter().position(|a| a.name == prev_name)
                                                        {
                                                            self.artist_list_state.select(Some(new_idx));
                                                            // Load the restored artist's albums
                                                            if let Err(e) = library.load_artist(&client, new_idx).await {
                                                                log::warn!("Failed to load artist after refresh: {}", e);
                                                            }
                                                        } else if !library.artists.is_empty() {
                                                            // Artist no longer exists, select first
                                                            self.artist_list_state.select(Some(0));
                                                            if let Err(e) = library.load_artist(&client, 0).await {
                                                                log::warn!(
                                                                    "Failed to load first artist after refresh: {}",
                                                                    e
                                                                );
                                                            }
                                                        }
                                                    }
                                                } else if let Some(ref mut library) = self.library {
                                                    // No previous selection, select first if available
                                                    if !library.artists.is_empty() {
                                                        self.artist_list_state.select(Some(0));
                                                        if let Err(e) = library.load_artist(&client, 0).await {
                                                            log::warn!("Failed to load first artist after refresh: {}", e);
                                                        }
                                                    }
                                                }

                                                // Clear album selections since they may be stale
                                                self.album_list_state.select(None);
                                                self.album_display_list_state.select(None);
                                                self.expanded_albums.clear();

                                                // Mark library as dirty for re-render
                                                self.dirty.mark_library();

                                            }
                                            Err(e) => {
                                                log::error!("Failed to refresh library: {}", e);

                                                // Show error only if user initiated the update
                                                if was_user_initiated {  // ← Changed from self.update_in_progress
                                                    self.set_status_message(StatusMessage {
                                                        text: e.to_string(),
                                                        created_at: std::time::Instant::now(),
                                                        message_type: MessageType::Error,
                                                    });
                                                }
                                            }
                                        }

                                        self.run_updates(&client).await?;

                                        self.library_reload_pending = false;
                                        self.pending_artist_index = None;
                                    } else {
                                        log::debug!("Database update completed (external), reloading silently...");

                                        // Get current artist name (if any) to try to restore after reload
                                        let current_artist_name = self.artist_list_state.selected()
                                            .and_then(|idx| self.library.as_ref()?.artists.get(idx).map(|a| a.name.clone()));

                                        match LazyLibrary::init(&client).await {
                                            Ok(new_library) => {
                                                self.library = Some(new_library);

                                                // Restore artist selection (find by name to handle removals/renames)
                                                if let Some(ref name) = current_artist_name {
                                                    if let Some(ref mut library) = self.library {
                                                        if let Some(new_idx) = library.artists.iter().position(|a| &a.name == name) {
                                                            self.artist_list_state.select(Some(new_idx));
                                                            // Artist still exists, keep album selections
                                                        } else {
                                                            // Artist no longer exists, select first and clear album selections
                                                            if !library.artists.is_empty() {
                                                                self.artist_list_state.select(Some(0));
                                                            }
                                                            self.album_list_state.select(None);
                                                            self.album_display_list_state.select(None);
                                                            self.expanded_albums.clear();
                                                        }
                                                    }
                                                } else if let Some(ref mut library) = self.library {
                                                    // No artist was selected, select first
                                                    if !library.artists.is_empty() {
                                                        self.artist_list_state.select(Some(0));
                                                        if let Err(e) = library.load_artist(&client, 0).await {
                                                            log::warn!("Failed to load first artist after refresh: {}", e);
                                                        }
                                                    }
                                                }

                                                // Mark library as dirty for re-render
                                                self.dirty.mark_library();
                                            }
                                            Err(e) => {
                                                log::error!("Failed to reload library after external update: {}", e);
                                            }
                                        }
                                    }
                                }
                                // Database, output, sticker, etc. - typically don't affect current playback
                                Subsystem::Database
                                | Subsystem::Output
                                | Subsystem::Sticker
                                | Subsystem::Subscription
                                | Subsystem::Message
                                | Subsystem::Partition
                                | Subsystem::Neighbor
                                | Subsystem::Mount
                                | Subsystem::Other(_) => {
                                    // These don't typically require UI updates
                                    log::debug!("Ignoring subsystem change: {:?}", subsystem);
                                }
                                // Catch-all for any future subsystem types
                                _ => {
                                    log::debug!("Unknown subsystem change: {:?}, doing full update", subsystem);
                                    self.run_updates(&client).await?;
                                }
                            }

                            // Check for song change
                            check_song_change(
                                &mut current_song_file,
                                &self.current_song,
                                &self.queue,
                                &client,
                                &cover_tx,
                                &mut protocol,
                                cover_cache.clone(),
                            );

                            // Handle PipeWire sample rate changes
                            #[cfg(target_os = "linux")]
                            handle_pipewire_state_change(
                                &self.config,
                                self.bit_perfect_enabled,
                                &self.mpd_status,
                                &self.current_song,
                                &mut self.last_play_state,
                                &mut self.last_sample_rate,
                            );
                        }
                        Some(ConnectionEvent::ConnectionClosed(err)) => {
                            log::error!("MPD connection closed: {:?}", err);
                            self.running = false;
                        }
                        None => {
                            log::info!("MPD connection closed cleanly");
                            self.running = false;
                        }
                    }
                }

                // Progress bar updates (only when playing)
                _ = progress_interval.tick() => {
                    // Only fetch status for progress updates when playing
                    if let Some(ref status) = self.mpd_status
                        && status.state == PlayState::Playing
                    {
                        // Just update status for progress bar, not full update
                        if let Ok(new_status) = client.command(mpd_client::commands::Status).await {
                            let progress = match (new_status.elapsed, new_status.duration) {
                                (Some(elapsed), Some(duration)) => {
                                    Some(elapsed.as_secs_f64() / duration.as_secs_f64())
                                }
                                _ => None,
                            };

                            if let Some(ref mut song) = self.current_song {
                                song.update_playback_info(Some(new_status.state), progress);
                                song.update_time_info(new_status.elapsed, new_status.duration);
                            }
                            self.mpd_status = Some(new_status);

                            // Mark progress as dirty to trigger redraw
                            self.dirty.mark_progress();
                        }
                    }
                }

                // Cover art loading results
                Some(msg) = cover_rx.recv() => {
                    match msg {
                        CoverArtMessage::Loaded(data, file_path) => {
                            // Only update if this is still the current song
                            if current_song_file.as_ref() == Some(&file_path) {
                                protocol.image = data
                                    .as_ref()
                                    .and_then(|raw_data| {
                                        image::ImageReader::new(Cursor::new(raw_data))
                                            .with_guessed_format()
                                            .ok()
                                    })
                                    .and_then(|reader| reader.decode().ok())
                                    .map(|dyn_img| picker.new_resize_protocol(dyn_img));

                                // Mark cover art as dirty to trigger redraw
                                self.dirty.mark_cover_art();
                                log::debug!("Cover art loaded for {:?}", file_path);
                            }
                        }
                    }
                }
            }

            // Check for Unix signals outside of select! to avoid conditional compilation issues
            #[cfg(unix)]
            {
                use std::pin::Pin;
                use std::task::Poll;

                let waker = futures::task::noop_waker();
                let mut cx = std::task::Context::from_waker(&waker);

                if let Poll::Ready(Some(())) = Pin::new(&mut sigint).poll_recv(&mut cx) {
                    log::info!("Received SIGINT, shutting down gracefully");
                    self.quit();
                }

                if let Poll::Ready(Some(())) = Pin::new(&mut sigterm).poll_recv(&mut cx) {
                    log::info!("Received SIGTERM, shutting down gracefully");
                    self.quit();
                }
            }
        }

        log::info!("Exiting main loop");

        // Reset PipeWire sample rate on exit
        #[cfg(target_os = "linux")]
        if self.bit_perfect_enabled && self.config.pipewire.is_available() {
            log::debug!("Resetting PipeWire sample rate on exit");
            let _ = pipewire::reset_sample_rate_async().await;
        }

        Ok(())
    }
}
