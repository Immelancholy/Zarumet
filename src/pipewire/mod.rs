//! PipeWire integration module
//!
//! Provides functionality to control PipeWire sample rate based on
//! the currently playing song in MPD.

mod pipewire;

pub use pipewire::{reset_sample_rate, set_sample_rate};
