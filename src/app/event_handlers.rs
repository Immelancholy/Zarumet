use crossterm::event::{Event, KeyEvent, KeyEventKind};
use mpd_client::Client;

use super::App;
use crate::app::constructor::save_bit_perfect_state;
use crate::app::mpd_handler::MPDAction;
use crate::app::navigation::Navigation;
use crate::logging::log_user_interaction;

/// Trait for event handling
pub trait EventHandlers {
    async fn handle_crossterm_events(&mut self, client: &Client) -> color_eyre::Result<()>;
    async fn on_key_event(&mut self, key: KeyEvent, client: &Client) -> color_eyre::Result<()>;
    fn quit(&mut self);
}

impl EventHandlers for App {
    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self, client: &Client) -> color_eyre::Result<()> {
        // Try direct event reading to bypass any terminal interference
        match crossterm::event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key, client).await?;
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn on_key_event(&mut self, key: KeyEvent, client: &Client) -> color_eyre::Result<()> {
        // Handle config warnings popup - any key closes it
        if self.show_config_warnings_popup {
            self.show_config_warnings_popup = false;
            return Ok(());
        }

        // Track whether we were awaiting input before handling the key
        let was_awaiting = self.key_binds.is_awaiting_input();

        if let Some(action) = self
            .key_binds
            .handle_key(key, &self.menu_mode, &self.panel_focus)
        {
            // Log user interaction with menu context
            let context = format!("menu:{:?}, panel:{:?}", self.menu_mode, self.panel_focus);
            log_user_interaction(&action.to_string(), Some(&context));

            // Check if this action modifies MPD state (requires immediate status refresh)
            let needs_update = matches!(
                action,
                MPDAction::TogglePlayPause
                    | MPDAction::Next
                    | MPDAction::Previous
                    | MPDAction::Random
                    | MPDAction::Repeat
                    | MPDAction::Single
                    | MPDAction::Consume
                    | MPDAction::VolumeUp
                    | MPDAction::VolumeUpFine
                    | MPDAction::VolumeDown
                    | MPDAction::VolumeDownFine
                    | MPDAction::ToggleMute
                    | MPDAction::SeekForward
                    | MPDAction::SeekBackward
                    | MPDAction::ClearQueue
                    | MPDAction::RemoveFromQueue
                    | MPDAction::MoveUpInQueue
                    | MPDAction::MoveDownInQueue
                    | MPDAction::PlaySelected
                    | MPDAction::AddSongToQueue
                    | MPDAction::ToggleAlbumExpansion
            );

            match action {
                MPDAction::Quit => self.quit(),
                MPDAction::ToggleBitPerfect => {
                    // Only allow toggling if bit-perfect is available (enabled in config)
                    if self.config.pipewire.is_available() {
                        self.bit_perfect_enabled = !self.bit_perfect_enabled;
                        self.dirty.mark_status();
                        // Reset PipeWire state tracking so handle_pipewire_state_change
                        // will properly detect state changes after toggle
                        self.last_play_state = None;
                        self.last_sample_rate = None;

                        #[cfg(target_os = "linux")]
                        if self.bit_perfect_enabled {
                            // Enabling - set sample rate if currently playing
                            if let Some(ref status) = self.mpd_status
                                && status.state == mpd_client::responses::PlayState::Playing
                                && let Some(ref song) = self.current_song
                                && let Some(song_rate) = song.sample_rate()
                                && let Some(supported_rates) =
                                    crate::pipewire::get_supported_rates()
                            {
                                let target_rate = crate::config::resolve_bit_perfect_rate(
                                    song_rate,
                                    &supported_rates,
                                );
                                // Fire-and-forget async call
                                tokio::spawn(async move {
                                    let _ =
                                        crate::pipewire::set_sample_rate_async(target_rate).await;
                                });
                            }
                        } else {
                            // Disabling - reset PipeWire sample rate to automatic
                            // We need to wait for reset before pause/unpause to force renegotiation
                            let is_playing = self.mpd_status.as_ref().is_some_and(|s| {
                                s.state == mpd_client::responses::PlayState::Playing
                            });
                            if crate::pipewire::reset_sample_rate_async().await.is_ok()
                                && is_playing
                            {
                                // Do a quick pause/unpause to force PipeWire to renegotiate
                                let _ = client.command(mpd_client::commands::SetPause(true)).await;
                                let _ = client.command(mpd_client::commands::Play::current()).await;
                            }
                        }
                    }
                }
                MPDAction::Next | MPDAction::Previous => {
                    // Only allow Next/Previous if queue is not empty
                    if !self.queue.is_empty() {
                        self.handle_navigation_action(action, client).await?;
                    }
                }
                _ => {
                    // Handle other actions through navigation trait
                    self.handle_navigation_action(action, client).await?;
                }
            }

            // Force immediate MPD status update for actions that modify state
            if needs_update {
                self.force_update = true;
            }
        }

        // Mark key sequence dirty if awaiting state changed (either started or ended a sequence)
        if was_awaiting || self.key_binds.is_awaiting_input() {
            self.dirty.mark_key_sequence();
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        // Save bit-perfect state before quitting
        let _ = save_bit_perfect_state(self.bit_perfect_enabled);
        self.running = false;
    }
}
