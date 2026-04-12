/// Menu modes for the application
#[derive(Debug, Clone, PartialEq)]
pub enum MenuMode {
    Queue,
    Artists,
    Albums,
    Years,
    Genres,
    Uris,
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
    UriList,
    UriAlbums,
}
