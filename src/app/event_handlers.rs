use crossterm::event::{Event, KeyEvent, KeyEventKind};
use mpd_client::Client;

use super::App;
use crate::app::mpd_handler::MPDAction;
use crate::app::navigation::Navigation;

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
        if let Some(action) = self
            .key_binds
            .handle_key(key, &self.menu_mode, &self.panel_focus)
        {
            match action {
                MPDAction::Quit => self.quit(),
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
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
