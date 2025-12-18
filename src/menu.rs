/// Menu modes for the application
#[derive(Debug, Clone, PartialEq)]
pub enum MenuMode {
    Queue,
    Tracks,
}

/// Panel focus for Tracks mode
#[derive(Debug, Clone, PartialEq)]
pub enum PanelFocus {
    Artists,
    Albums,
}
