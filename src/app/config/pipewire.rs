use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PipewireConfig {
    /// Enable bit-perfect mode for PipeWire
    #[serde(default = "PipewireConfig::default_bit_perfect_enabled")]
    pub bit_perfect_enabled: bool,
}

impl PipewireConfig {
    fn default_bit_perfect_enabled() -> bool {
        false
    }

    /// Check if bit-perfect mode is available (enabled and on Linux)
    pub fn is_available(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.bit_perfect_enabled
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

/// Get the best sample rate to use based on the song's sample rate
/// and the supported rates from PipeWire.
///
/// Logic:
/// 1. If the song rate is directly supported, use it
/// 2. Otherwise, find the highest supported rate that the song's rate is a multiple of
///    (i.e., song_rate % supported_rate == 0, meaning supported_rate divides evenly into song_rate)
///    e.g., for 192000 song with supported [44100, 48000, 96000], pick 96000 since 192000 % 96000 == 0
/// 3. Fallback to 44100 if no compatible rate is found
pub fn resolve_bit_perfect_rate(song_rate: u32, supported_rates: &[u32]) -> u32 {
    // If the song rate is directly supported, use it
    if supported_rates.contains(&song_rate) {
        return song_rate;
    }

    // Find the highest supported rate that divides evenly into the song's rate
    // (i.e., song_rate % supported_rate == 0)
    let best_rate = supported_rates
        .iter()
        .filter(|&&rate| song_rate.is_multiple_of(rate))
        .max()
        .copied();

    if let Some(rate) = best_rate {
        return rate;
    }

    // Fallback: prefer 44100 if available, otherwise first supported rate or 44100
    if supported_rates.contains(&44100) {
        44100
    } else {
        supported_rates.first().copied().unwrap_or(44100)
    }
}

impl Default for PipewireConfig {
    fn default() -> Self {
        Self {
            bit_perfect_enabled: Self::default_bit_perfect_enabled(),
        }
    }
}
