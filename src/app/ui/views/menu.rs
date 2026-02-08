/// Menu modes for the application
#[derive(Debug, Clone, PartialEq)]
pub enum MenuMode {
    Queue,
    Artists,
    Albums,
    Years,
    Genres,
}

/// Panel focus for Tracks mode
#[derive(Debug, Clone, PartialEq)]
pub enum PanelFocus {
    Artists,
    Albums,
    AlbumList,
    AlbumTracks,
    YearList,
    YearAlbums,
    GenreList,
    GenreAlbums,
}
