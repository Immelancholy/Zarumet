use crate::config::Config;
use crate::logging::log_mpd_command;
use mpd_client::{
    client::CommandError,
    commands,
    responses::{PlayState, Status},
};
use std::fmt;

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
    VolumeUpFine,
    VolumeDown,
    VolumeDownFine,
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
    SwitchToArtists,
    SwitchToAlbums,

    // Panel focus
    SwitchPanelLeft,
    SwitchPanelRight,

    // Panel-specific navigation
    NavigateUp,
    NavigateDown,

    // Album expansion
    ToggleAlbumExpansion,
    AddSongToQueue,

    // Mode cycling
    CycleModeLeft,
    CycleModeRight,

    // Scrolling
    ScrollUp,
    ScrollDown,

    // Jump to top/bottom
    GoToTop,
    GoToBottom,

    // PipeWire bit-perfect mode
    ToggleBitPerfect,
}

impl fmt::Display for MPDAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MPDAction::TogglePlayPause => write!(f, "TogglePlayPause"),
            MPDAction::Next => write!(f, "Next"),
            MPDAction::Previous => write!(f, "Previous"),
            MPDAction::Random => write!(f, "Random"),
            MPDAction::Repeat => write!(f, "Repeat"),
            MPDAction::Single => write!(f, "Single"),
            MPDAction::Consume => write!(f, "Consume"),
            MPDAction::VolumeUp => write!(f, "VolumeUp"),
            MPDAction::VolumeUpFine => write!(f, "VolumeUpFine"),
            MPDAction::VolumeDown => write!(f, "VolumeDown"),
            MPDAction::VolumeDownFine => write!(f, "VolumeDownFine"),
            MPDAction::ToggleMute => write!(f, "ToggleMute"),
            MPDAction::SeekForward => write!(f, "SeekForward"),
            MPDAction::SeekBackward => write!(f, "SeekBackward"),
            MPDAction::ClearQueue => write!(f, "ClearQueue"),
            MPDAction::RemoveFromQueue => write!(f, "RemoveFromQueue"),
            MPDAction::MoveUpInQueue => write!(f, "MoveUpInQueue"),
            MPDAction::MoveDownInQueue => write!(f, "MoveDownInQueue"),
            MPDAction::QueueUp => write!(f, "QueueUp"),
            MPDAction::QueueDown => write!(f, "QueueDown"),
            MPDAction::PlaySelected => write!(f, "PlaySelected"),
            MPDAction::Quit => write!(f, "Quit"),
            MPDAction::Refresh => write!(f, "Refresh"),
            MPDAction::SwitchToQueueMenu => write!(f, "SwitchToQueueMenu"),
            MPDAction::SwitchToArtists => write!(f, "SwitchToArtists"),
            MPDAction::SwitchToAlbums => write!(f, "SwitchToAlbums"),
            MPDAction::SwitchPanelLeft => write!(f, "SwitchPanelLeft"),
            MPDAction::SwitchPanelRight => write!(f, "SwitchPanelRight"),
            MPDAction::NavigateUp => write!(f, "NavigateUp"),
            MPDAction::NavigateDown => write!(f, "NavigateDown"),
            MPDAction::ToggleAlbumExpansion => write!(f, "ToggleAlbumExpansion"),
            MPDAction::AddSongToQueue => write!(f, "AddSongToQueue"),
            MPDAction::CycleModeLeft => write!(f, "CycleModeLeft"),
            MPDAction::CycleModeRight => write!(f, "CycleModeRight"),
            MPDAction::ScrollUp => write!(f, "ScrollUp"),
            MPDAction::ScrollDown => write!(f, "ScrollDown"),
            MPDAction::GoToTop => write!(f, "GoToTop"),
            MPDAction::GoToBottom => write!(f, "GoToBottom"),
            MPDAction::ToggleBitPerfect => write!(f, "ToggleBitPerfect"),
        }
    }
}

impl MPDAction {
    /// Returns true if this action sends commands to MPD
    fn is_mpd_command(&self) -> bool {
        matches!(
            self,
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
        )
    }

    /// Execute the action on the MPD client
    ///
    /// Uses the cached status when available to avoid extra MPD round-trips.
    pub async fn execute(
        &self,
        client: &mpd_client::Client,
        config: &Config,
        cached_status: Option<&Status>,
    ) -> Result<(), CommandError> {
        let result = self.execute_inner(client, config, cached_status).await;

        // Log MPD commands (not UI-only actions)
        if self.is_mpd_command() {
            match &result {
                Ok(()) => log_mpd_command(&self.to_string(), true, None),
                Err(e) => log_mpd_command(&self.to_string(), false, Some(&e.to_string())),
            }
        }

        result
    }

    async fn execute_inner(
        &self,
        client: &mpd_client::Client,
        config: &Config,
        cached_status: Option<&Status>,
    ) -> Result<(), CommandError> {
        match self {
            MPDAction::TogglePlayPause => {
                // Use cached status if available, otherwise fetch
                let current_state = if let Some(status) = cached_status {
                    status.state
                } else {
                    client.command(commands::Status).await?.state
                };
                match current_state {
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
                let increment = config.mpd.volume_increment as u8;
                // Use cached status if available
                let current_volume = if let Some(status) = cached_status {
                    status.volume
                } else {
                    client.command(commands::Status).await?.volume
                };
                let new_volume = std::cmp::min(100, current_volume + increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeUpFine => {
                let increment = config.mpd.volume_increment_fine as u8;
                let current_volume = if let Some(status) = cached_status {
                    status.volume
                } else {
                    client.command(commands::Status).await?.volume
                };
                let new_volume = std::cmp::min(100, current_volume + increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeDown => {
                let increment = config.mpd.volume_increment as u8;
                let current_volume = if let Some(status) = cached_status {
                    status.volume
                } else {
                    client.command(commands::Status).await?.volume
                };
                let new_volume = current_volume.saturating_sub(increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeDownFine => {
                let increment = config.mpd.volume_increment_fine as u8;
                let current_volume = if let Some(status) = cached_status {
                    status.volume
                } else {
                    client.command(commands::Status).await?.volume
                };
                let new_volume = current_volume.saturating_sub(increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::ToggleMute => {
                let current_volume = if let Some(status) = cached_status {
                    status.volume
                } else {
                    client.command(commands::Status).await?.volume
                };
                if current_volume > 0 {
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
                let random = if let Some(status) = cached_status {
                    status.random
                } else {
                    client.command(commands::Status).await?.random
                };
                client.command(commands::SetRandom(!random)).await?;
            }
            MPDAction::Repeat => {
                let repeat = if let Some(status) = cached_status {
                    status.repeat
                } else {
                    client.command(commands::Status).await?.repeat
                };
                client.command(commands::SetRepeat(!repeat)).await?;
            }
            MPDAction::Single => {
                let single = if let Some(status) = cached_status {
                    status.single
                } else {
                    client.command(commands::Status).await?.single
                };
                // Toggle single mode
                let new_single = match single {
                    commands::SingleMode::Enabled => commands::SingleMode::Disabled,
                    _ => commands::SingleMode::Enabled,
                };
                client.command(commands::SetSingle(new_single)).await?;
            }
            MPDAction::Consume => {
                let consume = if let Some(status) = cached_status {
                    status.consume
                } else {
                    client.command(commands::Status).await?.consume
                };
                client.command(commands::SetConsume(!consume)).await?;
            }
            MPDAction::QueueUp
            | MPDAction::QueueDown
            | MPDAction::PlaySelected
            | MPDAction::Quit
            | MPDAction::Refresh
            | MPDAction::MoveUpInQueue
            | MPDAction::MoveDownInQueue
            | MPDAction::SwitchToQueueMenu
            | MPDAction::SwitchToArtists
            | MPDAction::SwitchToAlbums
            | MPDAction::SwitchPanelLeft
            | MPDAction::SwitchPanelRight
            | MPDAction::NavigateUp
            | MPDAction::NavigateDown
            | MPDAction::ToggleAlbumExpansion
            | MPDAction::AddSongToQueue
            | MPDAction::CycleModeLeft
            | MPDAction::CycleModeRight
            | MPDAction::ScrollUp
            | MPDAction::ScrollDown
            | MPDAction::GoToTop
            | MPDAction::GoToBottom
            | MPDAction::ToggleBitPerfect => {
                // These are handled by the main application
            }
        }
        Ok(())
    }
}
