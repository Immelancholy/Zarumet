mod binds;
mod cli;
mod config;
mod menu;
mod song;
mod ui;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::executor::block_on;
use mpd_client::{Client, commands};
use ratatui::{DefaultTerminal, widgets::ListState};
use ratatui_image::picker::Picker;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpStream;

use binds::{KeyBinds, MPDAction};
use cli::Args;
use config::Config;
use menu::MenuMode;
use song::{SongInfo, Library, Album, Artist};
use ui::Protocol;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Initialize terminal with explicit crossterm configuration for full control
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let terminal =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    let result = App::new(args)?.run(terminal).await;

    // Restore terminal
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// Current song information
    current_song: Option<SongInfo>,
    /// MPD queue information
    queue: Vec<SongInfo>,
    /// Currently selected queue item index
    selected_queue_index: Option<usize>,
    /// List state for the queue widget
    queue_list_state: ListState,
    /// List states for Tracks navigation
    artist_list_state: ListState,
    album_list_state: ListState,
    track_list_state: ListState,
    /// Configuration loaded from TOML file
    config: Config,
    /// Current menu mode
    menu_mode: MenuMode,
    /// Music library
    library: Option<Library>,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new(args: Args) -> color_eyre::Result<Self> {
        let mut config = Config::load(args.config)?;

        if let Some(address) = args.address {
            config.mpd.address = address;
        }

        let queue_list_state = ListState::default();
        // Don't select anything initially - will be set when queue is populated

        Ok(Self {
            running: false,
            current_song: None,
            queue: Vec::new(),
            selected_queue_index: None, // Will be set when queue is populated
            queue_list_state,
            artist_list_state: ListState::default(),
            album_list_state: ListState::default(),
            track_list_state: ListState::default(),
            config,
            menu_mode: MenuMode::Queue, // Start with queue menu
            library: None,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;

        // Connect to MPD
        let connection = TcpStream::connect(&self.config.mpd.address).await?;
        let (client, _state_changes) = Client::connect(connection).await?;

        match SongInfo::set_max_art_size(&client, 5 * 1024 * 1024).await {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to set MPD binary limit: {}", e),
        }

        // Load library
        self.library = Some(Library::load_library(&client).await?);
        
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
                ui::render(
                    frame,
                    &mut protocol,
                    &self.current_song,
                    &self.queue,
                    &mut self.queue_list_state,
                    &self.config,
                     &self.menu_mode,
                     &self.library,
                     &mut self.artist_list_state,
                )
            })?;

            if let Some(ref mut img) = protocol.image {
                img.last_encoding_result();
            }

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

                current_song_file = new_song_file;
            }
        }
        Ok(())
    }

    /// Update the current song information from MPD
    async fn update_current_song(&mut self, client: &Client) -> color_eyre::Result<()> {
        match client.command(commands::CurrentSong).await {
            Ok(Some(song_in_queue)) => {
                self.current_song = Some(SongInfo::from_song(&song_in_queue.song));
            }
            Ok(None) => {
                self.current_song = None;
            }
            Err(_) => {
                // Keep the previous song info on error
            }
        }
        Ok(())
    }

    /// Run update functions concurrently with optimized result processing
    async fn run_updates(&mut self, client: &Client) -> color_eyre::Result<()> {
        // Run MPD commands concurrently
        let (current_song_result, queue_songs, status) = tokio::try_join!(
            client.command(commands::CurrentSong),
            client.command(commands::Queue),
            client.command(commands::Status)
        )?;

        // Process current song result
        match current_song_result {
            Some(song_in_queue) => {
                self.current_song = Some(SongInfo::from_song(&song_in_queue.song));
            }
            None => {
                self.current_song = None;
            }
        }

        // Process queue result
        self.queue = queue_songs
            .into_iter()
            .map(|song_in_queue| SongInfo::from_song(&song_in_queue.song))
            .collect();

        // Update selected index to stay within bounds and select first item if queue was previously empty
        match self.queue_list_state.selected() {
            Some(selected) => {
                // If we have a selection, keep it within bounds
                if selected >= self.queue.len() {
                    if self.queue.is_empty() {
                        self.queue_list_state.select(None);
                    } else {
                        self.queue_list_state
                            .select(Some(self.queue.len().saturating_sub(1)));
                    }
                }
            }
            None => {
                // If we have no selection and queue is not empty, select first item
                if !self.queue.is_empty() {
                    self.queue_list_state.select(Some(0));
                }
            }
        }
        self.selected_queue_index = self.queue_list_state.selected();

        // Process status result
        let progress = match (status.elapsed, status.duration) {
            (Some(elapsed), Some(duration)) => Some(elapsed.as_secs_f64() / duration.as_secs_f64()),
            _ => None,
        };

        if let Some(ref mut song) = self.current_song {
            song.update_playback_info(Some(status.state), progress);
            song.update_time_info(status.elapsed, status.duration);
        }

        Ok(())
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(
        &mut self,
        client: &mpd_client::Client,
    ) -> color_eyre::Result<()> {
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
    async fn on_key_event(
        &mut self,
        key: KeyEvent,
        client: &mpd_client::Client,
    ) -> color_eyre::Result<()> {
        if let Some(action) = KeyBinds::handle_key(key) {
            match action {
                MPDAction::QueueUp => {
                    match self.menu_mode {
                        MenuMode::Queue => {
                            if !self.queue.is_empty() {
                                let current = self.queue_list_state.selected().unwrap_or(0);
                                if current > 0 {
                                    self.queue_list_state.select(Some(current - 1));
                                } else {
                                    // Wrap around to the bottom
                                    self.queue_list_state
                                        .select(Some(self.queue.len().saturating_sub(1)));
                                }
                                self.selected_queue_index = self.queue_list_state.selected();
                            }
                        }
                        MenuMode::Tracks => {
                            if let Some(ref library) = self.library {
                                if !library.artists.is_empty() {
                                    let current = self.artist_list_state.selected().unwrap_or(0);
                                    if current > 0 {
                                        self.artist_list_state.select(Some(current - 1));
                                    } else {
                                        // Wrap around to the bottom
                                        self.artist_list_state
                                            .select(Some(library.artists.len().saturating_sub(1)));
                                    }
                                }
                            }
                        }
                    }
                }
                MPDAction::QueueDown => {
                    match self.menu_mode {
                        MenuMode::Queue => {
                            if !self.queue.is_empty() {
                                let current = self.queue_list_state.selected().unwrap_or(0);
                                if current < self.queue.len().saturating_sub(1) {
                                    self.queue_list_state.select(Some(current + 1));
                                } else {
                                    // Wrap around to the top
                                    self.queue_list_state.select(Some(0));
                                }
                                self.selected_queue_index = self.queue_list_state.selected();
                            }
                        }
                        MenuMode::Tracks => {
                            if let Some(ref library) = self.library {
                                if !library.artists.is_empty() {
                                    let current = self.artist_list_state.selected().unwrap_or(0);
                                    if current < library.artists.len().saturating_sub(1) {
                                        self.artist_list_state.select(Some(current + 1));
                                    } else {
                                        // Wrap around to the top
                                        self.artist_list_state.select(Some(0));
                                    }
                                }
                            }
                        }
                    }
                }
                MPDAction::PlaySelected => {
                    if let Some(selected) = self.queue_list_state.selected() {
                        if selected < self.queue.len() {
                            // Play the song at the selected position in the queue
                            let song_position: mpd_client::commands::SongPosition = selected.into();
                            if let Err(e) = client
                                .command(mpd_client::commands::Play::song(song_position))
                                .await
                            {
                                eprintln!("Error playing selected song: {}", e);
                            }
                        }
                    }
                }
                MPDAction::MoveUpInQueue => {
                    if let Some(selected) = self.queue_list_state.selected() {
                        if selected > 0 && selected < self.queue.len() {
                            // Move song up in queue (from position `selected` to `selected - 1`)
                            let from_pos: mpd_client::commands::SongPosition = selected.into();
                            let to_pos: mpd_client::commands::SongPosition = (selected - 1).into();
                            if let Err(e) = client
                                .command(
                                    mpd_client::commands::Move::position(from_pos)
                                        .to_position(to_pos),
                                )
                                .await
                            {
                                eprintln!("Error moving song up in queue: {}", e);
                            } else {
                                // Update selected index to follow the moved song
                                self.queue_list_state.select(Some(selected - 1));
                                self.selected_queue_index = self.queue_list_state.selected();
                            }
                        }
                    }
                }
                MPDAction::MoveDownInQueue => {
                    if let Some(selected) = self.queue_list_state.selected() {
                        if selected < self.queue.len().saturating_sub(1) {
                            // Move song down in queue (from position `selected` to `selected + 1`)
                            let from_pos: mpd_client::commands::SongPosition = selected.into();
                            let to_pos: mpd_client::commands::SongPosition = (selected + 1).into();
                            if let Err(e) = client
                                .command(
                                    mpd_client::commands::Move::position(from_pos)
                                        .to_position(to_pos),
                                )
                                .await
                            {
                                eprintln!("Error moving song down in queue: {}", e);
                            } else {
                                // Update selected index to follow the moved song
                                self.queue_list_state.select(Some(selected + 1));
                                self.selected_queue_index = self.queue_list_state.selected();
                            }
                        }
                    }
                }

                MPDAction::RemoveFromQueue => {
                    if let Some(selected) = self.queue_list_state.selected() {
                        if selected < self.queue.len() {
                            // Remove the selected song from queue
                            let song_position: mpd_client::commands::SongPosition = selected.into();
                            if let Err(e) = client
                                .command(mpd_client::commands::Delete::position(song_position))
                                .await
                            {
                                eprintln!("Error removing song from queue: {}", e);
                            } else {
                                // Update selected index to stay within bounds
                                if self.queue.is_empty() {
                                    self.queue_list_state.select(None);
                                } else if selected >= self.queue.len().saturating_sub(1) {
                                    self.queue_list_state
                                        .select(Some(self.queue.len().saturating_sub(1)));
                                }
                                self.selected_queue_index = self.queue_list_state.selected();
                            }
                        }
                    }
                }
                MPDAction::Quit => self.quit(),
                MPDAction::Refresh => {
                    // Force refresh by updating current song and queue
                    // This will be handled in the next update cycle
                }
                MPDAction::SwitchToQueueMenu => {
                    self.menu_mode = MenuMode::Queue;
                }
                MPDAction::SwitchToTracks => {
                    self.menu_mode = MenuMode::Tracks;
                }
                _ => {
                    // Execute MPD command for other actions
                    if let Err(e) = action.execute(client).await {
                        eprintln!("Error executing MPD command: {}", e);
                    }
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
