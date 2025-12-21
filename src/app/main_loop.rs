use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event;
use futures::executor::block_on;
use mpd_client::Client;
use mpd_client::responses::PlayState;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use tokio::net::TcpStream;

use super::App;
use crate::app::{event_handlers::EventHandlers, mpd_updates::MPDUpdates};
use crate::ui::Protocol;

/// Trait for main application loop
pub trait AppMainLoop {
    async fn run(self, terminal: DefaultTerminal) -> color_eyre::Result<()>
    where
        Self: Sized;
}

impl AppMainLoop for App {
    /// Run the application's main loop.
    async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;

        // Connect to MPD
        let connection = TcpStream::connect(&self.config.mpd.address).await?;
        let (client, _state_changes) = Client::connect(connection).await?;

        match crate::song::SongInfo::set_max_art_size(&client, 5 * 1024 * 1024).await {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to set MPD binary limit: {}", e),
        }

        // Load library
        self.library = Some(crate::song::Library::load_library(&client).await?);

        // Initialize artist selection if library has artists
        if let Some(ref library) = self.library {
            if !library.artists.is_empty() {
                self.artist_list_state.select(Some(0));
            }
        }

        // Set up the image picker and protocol
        let mut picker = Picker::from_query_stdio().unwrap();
        picker.set_background_color([0, 0, 0, 0]);

        // Fetch initial song info
        self.update_current_song(&client).await?;

        // Track the current song's file path (not the image path)
        let mut current_song_file: Option<PathBuf> = self
            .current_song
            .as_ref()
            .map(|song| song.file_path.clone());

        // Track playback state for PipeWire sample rate control
        let mut last_play_state: Option<PlayState> = None;

        // Try to get initial image
        let initial_image = self
            .current_song
            .as_ref()
            .and_then(|song| block_on(song.load_cover(&client)));

        // Create protocol with initial image (if available)
        let mut protocol = Protocol {
            image: initial_image
                .as_ref()
                .and_then(|raw_data| {
                    image::ImageReader::new(Cursor::new(raw_data))
                        .with_guessed_format()
                        .ok()
                })
                .and_then(|reader| reader.decode().ok())
                .map(|dyn_img| picker.new_resize_protocol(dyn_img)),
        };

        while self.running {
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

            // Poll for events with a timeout to allow periodic updates
            if event::poll(Duration::from_millis(10))? {
                self.handle_crossterm_events(&client).await?;
            }

            // Update song info, queue, and status periodically
            self.run_updates(&client).await?;

            // Check if the song changed (not just the image path)
            let new_song_file: Option<PathBuf> = self
                .current_song
                .as_ref()
                .map(|song| song.file_path.clone());

            if new_song_file != current_song_file {
                // Song changed, reload the cover art
                let new_image = self
                    .current_song
                    .as_ref()
                    .and_then(|song| block_on(song.load_cover(&client)));

                protocol.image = new_image
                    .as_ref()
                    .and_then(|raw_data| {
                        image::ImageReader::new(Cursor::new(raw_data))
                            .with_guessed_format()
                            .ok()
                    })
                    .and_then(|reader| reader.decode().ok())
                    .map(|dyn_img| picker.new_resize_protocol(dyn_img));

                // Update PipeWire sample rate if bit-perfect mode enabled, available, and playing
                if self.bit_perfect_enabled && self.config.pipewire.is_available() {
                    if let Some(ref status) = self.mpd_status {
                        if status.state == PlayState::Playing {
                            if let Some(ref song) = self.current_song {
                                if let Some(song_rate) = song.sample_rate() {
                                    let target_rate = self.config.pipewire.resolve_rate(song_rate);
                                    let _ = crate::pipewire::set_sample_rate(target_rate);
                                }
                            }
                        }
                    }
                }

                current_song_file = new_song_file;
            }

            // Handle PipeWire sample rate based on playback state changes
            if self.bit_perfect_enabled && self.config.pipewire.is_available() {
                let current_play_state = self.mpd_status.as_ref().map(|s| s.state);

                if current_play_state != last_play_state {
                    match current_play_state {
                        Some(PlayState::Playing) => {
                            // Started playing - set sample rate to match song
                            if let Some(ref song) = self.current_song {
                                if let Some(song_rate) = song.sample_rate() {
                                    let target_rate = self.config.pipewire.resolve_rate(song_rate);
                                    let _ = crate::pipewire::set_sample_rate(target_rate);
                                }
                            }
                        }
                        Some(PlayState::Paused) | Some(PlayState::Stopped) | None => {
                            // Paused or stopped - reset to automatic rate
                            if last_play_state == Some(PlayState::Playing) {
                                let _ = crate::pipewire::reset_sample_rate();
                            }
                        }
                    }
                    last_play_state = current_play_state;
                }
            }
        }
        Ok(())
    }
}
