//! PipeWire integration module
//!
//! Provides functionality to control PipeWire sample rate based on
//! the currently playing song in MPD.

#[allow(clippy::module_inception)]
mod pipewire;

pub use pipewire::{get_supported_rates, initialize_supported_rates, reset_sample_rate, set_sample_rate};
