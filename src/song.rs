use image::ImageReader;
use mpd_client::{Client, responses::Song};
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_dir: PathBuf,
    pub file_path: PathBuf,
}

impl SongInfo {
    pub fn from_song(song: &Song) -> Self {
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

        let file_path = song.file_path().to_path_buf();

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
            file_path,
        }
    }
    pub async fn load_cover(&self, client: &Client) -> Option<Vec<u8>> {
        let art_data = client
            .album_art(&self.file_path.to_string_lossy())
            .await
            .ok();

        if let Some(Some((data, Some(_mime_type)))) = art_data {
            let raw_data = data.to_vec();
            return Some(raw_data);
        } else {
            return None;
        }
    }
}
