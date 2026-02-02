use crate::app::SongInfo;
use crate::app::song::Album;

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone)]
pub(crate) enum ArtistData {
    NotLoaded,
    Loading,
    Loaded(Vec<Album>),
}

/// Lazy-loaded artist: initially only has the name, albums are loaded on demand
#[derive(Debug, Clone)]
pub struct LazyArtist {
    pub name: String,
    /// Albums for this artist - tracks loading state to prevent concurrent loads
    pub albums: ArtistData,
}

impl LazyArtist {
    /// Create a new lazy artist with just the name
    pub fn new(name: String) -> Self {
        let name = SongInfo::sanitize_string(&name);
        Self {
            name,
            albums: ArtistData::NotLoaded,
        }
    }

    /// Check if this artist's albums have been loaded
    pub fn is_loaded(&self) -> bool {
        matches!(self.albums, ArtistData::Loaded(_))
    }

    /// Check if this artist's albums are currently being loaded
    pub fn is_loading(&self) -> bool {
        matches!(self.albums, ArtistData::Loading)
    }

    /// Convert to a regular Artist (returns empty albums if not loaded)
    pub fn to_artist(&self) -> Artist {
        let albums = match &self.albums {
            ArtistData::Loaded(albums) => albums.clone(),
            _ => Vec::new(),
        };
        Artist {
            name: self.name.clone(),
            albums,
        }
    }
}
