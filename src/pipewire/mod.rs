//! PipeWire integration module
//!
//! Provides functionality to control PipeWire sample rate based on
//! the currently playing song in MPD.

#[allow(clippy::module_inception)]
mod pipewire;

pub use pipewire::{
    get_supported_rates, initialize_supported_rates, reset_sample_rate, set_sample_rate,
};

/// Async wrapper for set_sample_rate that runs the blocking PipeWire call
/// on a separate thread to avoid blocking the tokio runtime.
pub async fn set_sample_rate_async(rate: u32) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_sample_rate(rate))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
}

/// Async wrapper for reset_sample_rate that runs the blocking PipeWire call
/// on a separate thread to avoid blocking the tokio runtime.
pub async fn reset_sample_rate_async() -> Result<(), String> {
    tokio::task::spawn_blocking(reset_sample_rate)
        .await
        .map_err(|e| format!("Task join error: {e}"))?
}
