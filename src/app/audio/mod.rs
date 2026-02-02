//! PipeWire integration module
//!
//! Provides functionality to control PipeWire sample rate based on
//! the currently playing song in MPD.

#[cfg(target_os = "linux")]
pub mod pipewire;
