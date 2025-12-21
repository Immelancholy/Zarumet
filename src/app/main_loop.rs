use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

use mpd_client::Client;
use mpd_client::client::ConnectionEvent;
use mpd_client::responses::PlayState;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use super::App;
use crate::app::{event_handlers::EventHandlers, mpd_updates::MPDUpdates};
use crate::ui::Protocol;

/// Interval for progress bar updates when playing (in milliseconds)
const PROGRESS_UPDATE_INTERVAL_MS: u64 = 500;

/// Message type for cover art loading results
enum CoverArtMessage {
    Loaded(Option<Vec<u8>>, PathBuf),
}

/// Trait for main application loop
pub trait AppMainLoop {
    async fn run(self, terminal: DefaultTerminal) -> color_eyre::Result<()>
    where
        Self: Sized;
}

/// Connect to MPD via Unix socket or TCP based on address format
async fn connect_to_mpd(
    address: &str,
) -> color_eyre::Result<(Client, mpd_client::client::ConnectionEvents)> {
    let is_unix_socket = address.contains('/');

    if is_unix_socket {
        #[cfg(unix)]
        {
            let connection = UnixStream::connect(address).await?;
            Ok(Client::connect(connection).await?)
        }
        #[cfg(not(unix))]
        {
            Err(color_eyre::eyre::eyre!(
                "Unix sockets are not supported on this platform"
            ))
        }
    } else {
        let connection = TcpStream::connect(address).await?;
        Ok(Client::connect(connection).await?)
    }
}

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

        match crate::song::SongInfo::set_max_art_size(&client, 5 * 1024 * 1024).await {
            Ok(_) => {
                log::debug!("Set MPD binary limit to 5MB");
            }
            Err(e) => {
                log::warn!("Failed to set MPD binary limit: {}", e);
            }
        }

        // Load library
        self.library = Some(crate::song::Library::load_library(&client).await?);

        // Initialize artist selection if library has artists
        if let Some(ref library) = self.library
            && !library.artists.is_empty()
        {
            self.artist_list_state.select(Some(0));
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

        // Track playback state for PipeWire sample rate control
        #[allow(unused_variables)]
        let mut last_play_state: Option<PlayState> = None;
        #[allow(unused_variables)]
        let mut last_sample_rate: Option<u32> = None;

        // Channel for cover art loading results
        let (cover_tx, mut cover_rx) = mpsc::channel::<CoverArtMessage>(1);

        // Load initial cover art in background
        if let Some(ref song) = self.current_song {
            let file_path = song.file_path.clone();
            spawn_cover_art_loader(&client, file_path, cover_tx.clone());
        }

        // Create protocol with no initial image (will be loaded async)
        let mut protocol = Protocol { image: None };

        // Progress update interval
        let progress_interval =
            tokio::time::interval(Duration::from_millis(PROGRESS_UPDATE_INTERVAL_MS));
        tokio::pin!(progress_interval);

        log::info!("Entering event-driven main loop");

        while self.running {
            // Render the UI
            terminal.draw(|frame| {
                crate::ui::render(
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
                    &self.panel_focus,
                    &self.expanded_albums,
                    &self.mpd_status,
                    &self.key_binds,
                    self.bit_perfect_enabled,
                )
            })?;

            if let Some(ref mut img) = protocol.image {
                img.last_encoding_result();
            }

            // Update key bindings for timeouts
            self.key_binds.update();

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
                                &client,
                                &cover_tx,
                            );
                        }
                    }
                }

                // MPD state change notifications
                mpd_event = state_changes.next() => {
                    match mpd_event {
                        Some(ConnectionEvent::SubsystemChange(subsystem)) => {
                            log::debug!("MPD subsystem change: {:?}", subsystem);

                            // Update state based on what changed
                            self.run_updates(&client).await?;

                            // Check for song change
                            check_song_change(
                                &mut current_song_file,
                                &self.current_song,
                                &client,
                                &cover_tx,
                            );

                            // Handle PipeWire sample rate changes
                            #[cfg(target_os = "linux")]
                            handle_pipewire_state_change(
                                &self.config,
                                self.bit_perfect_enabled,
                                &self.mpd_status,
                                &self.current_song,
                                &mut last_play_state,
                                &mut last_sample_rate,
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

                                log::debug!("Cover art loaded for {:?}", file_path);
                            }
                        }
                    }
                }
            }
        }

        log::info!("Exiting main loop");
        Ok(())
    }
}

/// Spawn a background task to load cover art
fn spawn_cover_art_loader(client: &Client, file_path: PathBuf, tx: mpsc::Sender<CoverArtMessage>) {
    let client = client.clone();
    let file_path_clone = file_path.clone();

    tokio::spawn(async move {
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

        // Send result back (ignore error if receiver dropped)
        let _ = tx
            .send(CoverArtMessage::Loaded(data, file_path_clone))
            .await;
    });
}

/// Check if the song changed and trigger cover art loading if needed
fn check_song_change(
    current_song_file: &mut Option<PathBuf>,
    current_song: &Option<crate::song::SongInfo>,
    client: &Client,
    cover_tx: &mpsc::Sender<CoverArtMessage>,
) {
    let new_song_file: Option<PathBuf> = current_song.as_ref().map(|song| song.file_path.clone());

    if new_song_file != *current_song_file {
        log::debug!(
            "Song changed: {:?} -> {:?}",
            current_song_file,
            new_song_file
        );

        // Start loading cover art in background
        if let Some(ref file_path) = new_song_file {
            spawn_cover_art_loader(client, file_path.clone(), cover_tx.clone());
        }

        *current_song_file = new_song_file;
    }
}

/// Handle PipeWire sample rate changes based on playback state and song changes
#[cfg(target_os = "linux")]
fn handle_pipewire_state_change(
    config: &crate::config::Config,
    bit_perfect_enabled: bool,
    mpd_status: &Option<mpd_client::responses::Status>,
    current_song: &Option<crate::song::SongInfo>,
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
                if let Ok(supported_rates) = crate::pipewire::get_supported_rates() {
                    let target_rate = crate::config::resolve_bit_perfect_rate(song_rate, &supported_rates);
                    log::debug!(
                        "Setting PipeWire sample rate to {} (song rate: {})",
                        target_rate,
                        song_rate
                    );
                    let _ = crate::pipewire::set_sample_rate(target_rate);
                }
            }
        }
        Some(PlayState::Paused) | Some(PlayState::Stopped) | None => {
            // Paused or stopped - reset to automatic rate
            if *last_play_state == Some(PlayState::Playing) {
                log::debug!("Resetting PipeWire sample rate (playback stopped)");
                let _ = crate::pipewire::reset_sample_rate();
            }
        }
    }

    *last_play_state = current_play_state;
    *last_sample_rate = current_sample_rate;
}
