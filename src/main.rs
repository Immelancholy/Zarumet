use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::imageops::FilterType;
use mpd_client::{Client, commands, responses::Song};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal).await;
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// Current song information
    current_song: Option<SongInfo>,
}

#[derive(Debug, Clone)]
pub struct SongInfo {
    title: String,
    artist: String,
    album: String,
    album_dir: PathBuf,
}

impl SongInfo {
    fn from_song(song: &Song) -> Self {
        let title = song
            .title()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Title".to_string());
        let artist = song
            .artists()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let album = song
            .album()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown Album".to_string());

        let album_dir = song
            .file_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        Self {
            title,
            artist,
            album,
            album_dir,
        }
    }
    pub fn find_cover_art(&self) -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let music_dir = PathBuf::from(home).join("Music");
        let full_album_path = music_dir.join(&self.album_dir);

        let cover_names = ["cover.jpg", "cover.png", "Cover.jpg", "Cover.png"];
        for name in cover_names {
            let cover_path = full_album_path.join(name);
            if cover_path.exists() {
                return Some(cover_path);
            }
        }
        None
    }
}

struct Protocol {
    image: StatefulProtocol,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;

        // Connect to MPD
        let connection = TcpStream::connect("localhost:6600").await?;
        let (client, _state_changes) = Client::connect(connection).await?;

        // Set up the image picker and protocol
        let mut picker = Picker::from_query_stdio().unwrap();
        picker.set_background_color([0, 0, 0, 0]);

        // Fetch initial song info
        self.update_current_song(&client).await?;

        let mut current_image_path = self
            .current_song
            .as_ref()
            .and_then(|song| song.find_cover_art())
            .unwrap_or_default();

        // Create protocol with initial image
        let dyn_img = image::ImageReader::open(&current_image_path)?.decode()?;
        let image = picker.new_resize_protocol(dyn_img);
        let mut protocol = Protocol { image };

        while self.running {
            terminal.draw(|frame| self.render(frame, &mut protocol))?;
            protocol.image.last_encoding_result();

            // Poll for events with a timeout to allow periodic updates
            if event::poll(Duration::from_millis(100))? {
                self.handle_crossterm_events()?;
            }

            // Update song info periodically
            self.update_current_song(&client).await?;

            let new_image_path = self
                .current_song
                .as_ref()
                .and_then(|song| song.find_cover_art())
                .unwrap_or_default();

            if new_image_path != current_image_path {
                if let Ok(reader) = image::ImageReader::open(&new_image_path) {
                    if let Ok(dyn_img) = reader.decode() {
                        protocol.image = picker.new_resize_protocol(dyn_img);
                        current_image_path = new_image_path;
                    }
                }
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

    /// Renders the user interface.
    fn render(&mut self, frame: &mut Frame<'_>, protocol: &mut Protocol) {
        let area = frame.area();

        // Split the area: image on top, song info at bottom
        let chunks = Layout::vertical([
            Constraint::Min(10),   // Image takes most space
            Constraint::Length(5), // Song info takes 5 lines
        ])
        .split(area);

        // Render the album art image (centered)
        let image = StatefulImage::default().resize(Resize::Fit(Some(FilterType::Lanczos3)));

        let image_area = center_area(
            chunks[0],
            Constraint::Percentage(100),
            Constraint::Percentage(100),
        );
        frame.render_stateful_widget(image, image_area, &mut protocol.image);

        // Render the song information
        let song_widget = self.create_song_widget(chunks[1]);
        frame.render_widget(song_widget, chunks[1]);
    }

    /// Create the song information widget
    fn create_song_widget(&self, _area: Rect) -> Paragraph<'_> {
        let lines = match &self.current_song {
            Some(song) => vec![
                Line::from(vec![
                    Span::styled("Title: ", Style::default().bold()),
                    Span::raw(&song.title),
                ]),
                Line::from(vec![
                    Span::styled("Artist: ", Style::default().bold()),
                    Span::raw(&song.artist),
                ]),
                Line::from(vec![
                    Span::styled("Album: ", Style::default().bold()),
                    Span::raw(&song.album),
                ]),
            ],
            None => vec![Line::from("No song playing").dark_gray()],
        };

        Paragraph::new(lines)
            .block(
                Block::default()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .title(" Now Playing "),
            )
            .centered()
    }

    /// Reads the crossterm events and updates the state of [`App`].
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}

/// Helper function to center a rect within another rect
fn center_area(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
