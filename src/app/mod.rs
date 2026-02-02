use crate::app::config::Config;
pub use crate::app::main_loop::AppMainLoop;
pub use crate::app::mpd::mpd_handler;
pub use crate::app::mpd::mpd_updates;
use crate::app::song::{LazyLibrary, SongInfo};
use crate::app::ui::{DirtyFlags, MenuMode, PanelFocus};
use binds_handler::KeyBinds;
pub use main::{App, MessageType, StatusMessage};
use mpd_client::responses::PlayState;
use ratatui::widgets::ListState;
use std::cell::Cell;

// Module declarations
pub mod audio;
pub mod binds_handler;
pub mod cli;
pub mod config;
pub mod constructor;
pub mod event_handlers;
pub mod logging;
pub mod main;
pub mod main_loop;
pub mod mpd;
pub mod navigation;
pub mod song;
pub mod terminal;
pub mod ui;
