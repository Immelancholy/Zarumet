use color_eyre::eyre::{Result, bail, eyre};
use mpd_client::{Client, commands};

use crate::app::song::SongInfo;

/// A fresh MPD snapshot for the stored-playlist browser, independent of queue versions.
pub struct PlaylistSnapshot {
    pub names: Vec<String>,
    pub selected: Option<String>,
    pub tracks: Result<Vec<SongInfo>, String>,
}

pub async fn browse(
    client: &Client,
    selected: Option<String>,
    index: usize,
) -> Result<PlaylistSnapshot, String> {
    let mut names: Vec<_> = client
        .command(commands::GetPlaylists)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|playlist| playlist.name)
        .collect();
    names.sort();
    let selected = selected
        .filter(|name| names.contains(name))
        .or_else(|| names.get(index.min(names.len().saturating_sub(1))).cloned());
    let tracks = if let Some(name) = &selected {
        client
            .command(commands::GetPlaylist(name))
            .await
            .map(|songs| songs.iter().map(SongInfo::from_song).collect())
            .map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    };
    Ok(PlaylistSnapshot {
        names,
        selected,
        tracks,
    })
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("Enter a playlist name");
    }
    if name
        .chars()
        .any(|c| c.is_control() || c == '/' || c == '\\')
    {
        bail!("Names cannot contain slashes or control characters");
    }
    Ok(())
}

pub async fn save_queue(client: &Client, name: &str) -> Result<()> {
    validate_name(name)?;
    // MPD's save command rejects collisions, including names created after the prompt opened.
    client.command(commands::SaveQueueAsPlaylist(name)).await?;
    Ok(())
}

pub async fn rename(client: &Client, from: &str, to: &str) -> Result<()> {
    validate_name(to)?;
    // MPD rejects an existing destination; never emulate rename with delete + save.
    client
        .command(commands::RenamePlaylist::new(from, to))
        .await?;
    Ok(())
}

pub async fn delete(client: &Client, name: &str) -> Result<()> {
    client.command(commands::DeletePlaylist(name)).await?;
    Ok(())
}

/// Stage individual entries before replacing anything. MPD command lists are not atomic.
pub async fn enqueue(client: &Client, name: &str, replace: bool) -> Result<usize> {
    let songs = client.command(commands::GetPlaylist(name)).await?;
    if songs.is_empty() {
        bail!("Playlist is empty; queue unchanged");
    }
    let old_queue = client.command(commands::Queue).await?;
    let old_ids: Vec<_> = old_queue.iter().map(|song| song.id).collect();
    let mut added = Vec::with_capacity(songs.len());
    for song in &songs {
        match client.command(commands::Add::uri(&song.url)).await {
            Ok(id) => added.push(id),
            Err(error) => {
                let cleanup = remove_added(client, &added).await;
                return Err(eyre!("Could not load playlist: {error}. {cleanup}"));
            }
        }
    }
    if replace {
        let current = match client.command(commands::Queue).await {
            Ok(queue) => queue,
            Err(error) => {
                let cleanup = remove_added(client, &added).await;
                return Err(eyre!("Could not verify queue: {error}. {cleanup}"));
            }
        };
        let expected: Vec<_> = old_ids.iter().chain(&added).copied().collect();
        if current.iter().map(|song| song.id).collect::<Vec<_>>() != expected {
            let cleanup = remove_added(client, &added).await;
            bail!("Queue changed while loading; replacement cancelled. {cleanup}");
        }
        // Use IDs: duplicate filenames and shifted positions must not remove the wrong entries.
        // A failure here can leave a partial replacement; report it and let the caller reconcile.
        for id in old_ids {
            client
                .command(commands::Delete::id(id))
                .await
                .map_err(|e| eyre!("Replacement incomplete: {e}. Check the refreshed queue"))?;
        }
        client
            .command(commands::Play::song(added[0]))
            .await
            .map_err(|e| eyre!("Playlist loaded but playback failed: {e}"))?;
    }
    Ok(added.len())
}

async fn remove_added(client: &Client, ids: &[commands::SongId]) -> &'static str {
    let mut failed = false;
    for id in ids {
        if client.command(commands::Delete::id(*id)).await.is_err() {
            failed = true;
        }
    }
    if failed {
        "Could not remove all staged entries; check the refreshed queue"
    } else {
        "Staged entries removed; original queue entries retained"
    }
}

#[cfg(test)]
mod tests {
    use super::validate_name;

    #[test]
    fn names_accept_unicode_and_quotes_but_reject_paths_and_controls() {
        for name in ["Été \"mix\"", "日本語", "a long playlist name", "mix.m3u"] {
            assert!(validate_name(name).is_ok());
        }
        for name in ["", "  ", "a/b", "a\\b", "bad\nname", "bad\0name"] {
            assert!(validate_name(name).is_err());
        }
    }
}
