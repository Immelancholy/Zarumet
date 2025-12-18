use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd_client::{client::CommandError, commands, responses::PlayState};
use crate::menu::{MenuMode, PanelFocus};

/// Key binding definitions for MPD controls
pub struct KeyBinds;

impl KeyBinds {
    /// Handle key events and return corresponding MPD commands
    pub fn handle_key(key: KeyEvent, mode: &MenuMode, panel_focus: &PanelFocus) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Global keybindings (work in all modes)
            (KeyModifiers::NONE, KeyCode::Char(' ')) => Some(MPDAction::TogglePlayPause),
            (KeyModifiers::NONE, KeyCode::Char('p')) => Some(MPDAction::TogglePlayPause),
            (KeyModifiers::NONE, KeyCode::Char('>')) | (KeyModifiers::NONE, KeyCode::Char('n')) => {
                Some(MPDAction::Next)
            }
            (KeyModifiers::NONE, KeyCode::Char('<')) | (KeyModifiers::NONE, KeyCode::Char('b')) => {
                Some(MPDAction::Previous)
            }
            (KeyModifiers::NONE, KeyCode::Char('=')) | (KeyModifiers::NONE, KeyCode::Char('+')) => {
                Some(MPDAction::VolumeUp)
            }
            (KeyModifiers::NONE, KeyCode::Char('-')) | (KeyModifiers::NONE, KeyCode::Char('_')) => {
                Some(MPDAction::VolumeDown)
            }
            (KeyModifiers::NONE, KeyCode::Char('m')) => Some(MPDAction::ToggleMute),
            (KeyModifiers::CONTROL, KeyCode::Char('l')) => Some(MPDAction::SeekForward),
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => Some(MPDAction::SeekBackward),
            (KeyModifiers::CONTROL, KeyCode::Right) => Some(MPDAction::SeekForward),
            (KeyModifiers::CONTROL, KeyCode::Left) => Some(MPDAction::SeekBackward),
            (KeyModifiers::NONE, KeyCode::Char('d')) => Some(MPDAction::ClearQueue),
            (KeyModifiers::NONE, KeyCode::Char('r')) => Some(MPDAction::Repeat),
            (KeyModifiers::NONE, KeyCode::Char('z')) => Some(MPDAction::Random),
            (KeyModifiers::NONE, KeyCode::Char('s')) => Some(MPDAction::Single),
            (KeyModifiers::NONE, KeyCode::Char('c')) => Some(MPDAction::Consume),
            (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
                Some(MPDAction::Quit)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('C')) => Some(MPDAction::Quit),
            (KeyModifiers::NONE, KeyCode::Char('R')) => Some(MPDAction::Refresh),
            (KeyModifiers::NONE, KeyCode::Char('1')) => Some(MPDAction::SwitchToQueueMenu),
            (KeyModifiers::NONE, KeyCode::Char('2')) => Some(MPDAction::SwitchToTracks),

            // Mode-specific keybindings
            _ => {
                match mode {
                    MenuMode::Queue => {
                        Self::handle_queue_mode_key(key)
                    }
                    MenuMode::Tracks => {
                        Self::handle_tracks_mode_key(key, panel_focus)
                    }
                }
            }
        }
    }

    /// Handle keys specific to Queue mode
    fn handle_queue_mode_key(key: KeyEvent) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Queue navigation
            (KeyModifiers::NONE, KeyCode::Char('j')) => Some(MPDAction::QueueDown),
            (KeyModifiers::NONE, KeyCode::Char('k')) => Some(MPDAction::QueueUp),
            (KeyModifiers::NONE, KeyCode::Down) => Some(MPDAction::QueueDown),
            (KeyModifiers::NONE, KeyCode::Up) => Some(MPDAction::QueueUp),
            (KeyModifiers::NONE, KeyCode::Enter) => Some(MPDAction::PlaySelected),
            (KeyModifiers::NONE, KeyCode::Char('l')) => Some(MPDAction::PlaySelected),
            (KeyModifiers::NONE, KeyCode::Right) => Some(MPDAction::PlaySelected),
            
            // Queue management
            (KeyModifiers::NONE, KeyCode::Char('x')) => Some(MPDAction::RemoveFromQueue),
            (KeyModifiers::CONTROL, KeyCode::Char('k')) => Some(MPDAction::MoveUpInQueue),
            (KeyModifiers::CONTROL, KeyCode::Char('j')) => Some(MPDAction::MoveDownInQueue),
            (KeyModifiers::CONTROL, KeyCode::Up) => Some(MPDAction::MoveUpInQueue),
            (KeyModifiers::CONTROL, KeyCode::Down) => Some(MPDAction::MoveDownInQueue),
            
            _ => None,
        }
    }

    /// Handle keys specific to Tracks mode
    fn handle_tracks_mode_key(key: KeyEvent, _panel_focus: &PanelFocus) -> Option<MPDAction> {
        match (key.modifiers, key.code) {
            // Panel switching
            (KeyModifiers::NONE, KeyCode::Char('h')) => Some(MPDAction::SwitchPanelLeft),
            (KeyModifiers::NONE, KeyCode::Char('l')) => Some(MPDAction::SwitchPanelRight),
            (KeyModifiers::NONE, KeyCode::Left) => Some(MPDAction::SwitchPanelLeft),
            (KeyModifiers::NONE, KeyCode::Right) => Some(MPDAction::SwitchPanelRight),
            
            // Navigation (up/down)
            (KeyModifiers::NONE, KeyCode::Char('j')) => Some(MPDAction::NavigateDown),
            (KeyModifiers::NONE, KeyCode::Char('k')) => Some(MPDAction::NavigateUp),
            (KeyModifiers::NONE, KeyCode::Down) => Some(MPDAction::NavigateDown),
            (KeyModifiers::NONE, KeyCode::Up) => Some(MPDAction::NavigateUp),
            
            // Action keys
            (KeyModifiers::NONE, KeyCode::Enter) => Some(MPDAction::PlaySelected),
            
            _ => None,
        }
    }
}

/// Actions that can be performed on MPD
#[derive(Debug, Clone)]
pub enum MPDAction {
    // Playback
    TogglePlayPause,
    Next,
    Previous,

    // Playback options
    Random,
    Repeat,
    Single,
    Consume,

    // Volume
    VolumeUp,
    VolumeDown,
    ToggleMute,

    // Seeking
    SeekForward,
    SeekBackward,

    // Queue options
    ClearQueue,
    RemoveFromQueue,
    MoveUpInQueue,
    MoveDownInQueue,

    // Queue navigation
    QueueUp,
    QueueDown,
    PlaySelected,

    // Application
    Quit,
    Refresh,

    // Menu mode
    SwitchToQueueMenu,
    SwitchToTracks,
    
    // Panel focus
    SwitchPanelLeft,
    SwitchPanelRight,
    
    // Panel-specific navigation
    NavigateUp,
    NavigateDown,
}

impl MPDAction {
    /// Execute the action on the MPD client
    pub async fn execute(&self, client: &mpd_client::Client) -> Result<(), CommandError> {
        match self {
            MPDAction::TogglePlayPause => {
                let status = client.command(commands::Status).await?;
                match status.state {
                    PlayState::Playing => {
                        client.command(commands::SetPause(true)).await?;
                    }
                    _ => {
                        client.command(commands::Play::current()).await?;
                    }
                }
            }
            MPDAction::Next => {
                client.command(commands::Next).await?;
            }
            MPDAction::Previous => {
                client.command(commands::Previous).await?;
            }
            MPDAction::VolumeUp => {
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::min(100, status.volume + 5);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeDown => {
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::max(0, status.volume - 5);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::ToggleMute => {
                let status = client.command(commands::Status).await?;
                if status.volume > 0 {
                    client.command(commands::SetVolume(0)).await?;
                } else {
                    client.command(commands::SetVolume(50)).await?;
                }
            }
            MPDAction::SeekForward => {
                client
                    .command(commands::Seek(commands::SeekMode::Forward(
                        std::time::Duration::from_secs(5),
                    )))
                    .await?;
            }
            MPDAction::SeekBackward => {
                client
                    .command(commands::Seek(commands::SeekMode::Backward(
                        std::time::Duration::from_secs(5),
                    )))
                    .await?;
            }
            MPDAction::ClearQueue => {
                client.command(commands::ClearQueue).await?;
            }
            MPDAction::RemoveFromQueue => {
                // This is handled by the main application since it needs the selected index
            }
            MPDAction::Random => {
                let status = client.command(commands::Status).await?;
                client.command(commands::SetRandom(!status.random)).await?;
            }
            MPDAction::Repeat => {
                let status = client.command(commands::Status).await?;
                client.command(commands::SetRepeat(!status.repeat)).await?;
            }
            MPDAction::Single => {
                let status = client.command(commands::Status).await?;
                // Toggle single mode
                let new_single = match status.single {
                    commands::SingleMode::Enabled => commands::SingleMode::Disabled,
                    _ => commands::SingleMode::Enabled,
                };
                client.command(commands::SetSingle(new_single)).await?;
            }
            MPDAction::Consume => {
                let status = client.command(commands::Status).await?;
                client
                    .command(commands::SetConsume(!status.consume))
                    .await?;
            }
            MPDAction::QueueUp
            | MPDAction::QueueDown
            | MPDAction::PlaySelected
            | MPDAction::Quit
            | MPDAction::Refresh
            | MPDAction::MoveUpInQueue
            | MPDAction::MoveDownInQueue
            | MPDAction::SwitchToQueueMenu
            | MPDAction::SwitchToTracks
            | MPDAction::SwitchPanelLeft
            | MPDAction::SwitchPanelRight
            | MPDAction::NavigateUp
            | MPDAction::NavigateDown => {
                // These are handled by the main application
            }
        }
        Ok(())
    }
}
