use crate::config::Config;
use mpd_client::{client::CommandError, commands, responses::PlayState};

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
    SwitchToTracks,

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
}

impl MPDAction {
    /// Execute the action on the MPD client
    pub async fn execute(
        &self,
        client: &mpd_client::Client,
        config: &Config,
    ) -> Result<(), CommandError> {
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
                let increment = config.mpd.volume_increment as u8;
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::min(100, status.volume + increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeUpFine => {
                let increment = config.mpd.volume_increment_fine as u8;
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::min(100, status.volume + increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeDown => {
                let increment = config.mpd.volume_increment as u8;
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::max(0, status.volume - increment);
                client.command(commands::SetVolume(new_volume)).await?;
            }
            MPDAction::VolumeDownFine => {
                let increment = config.mpd.volume_increment_fine as u8;
                let status = client.command(commands::Status).await?;
                let new_volume = std::cmp::max(0, status.volume - increment);
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
            | MPDAction::NavigateDown
            | MPDAction::ToggleAlbumExpansion
            | MPDAction::AddSongToQueue
            | MPDAction::CycleModeLeft
            | MPDAction::CycleModeRight
            | MPDAction::ScrollUp
            | MPDAction::ScrollDown => {
                // These are handled by the main application
            }
        }
        Ok(())
    }
}
