use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MpdConfig {
    #[serde(default = "MpdConfig::default_address")]
    pub address: String,
    #[serde(default = "MpdConfig::default_volume_increment")]
    pub volume_increment: u32,
    #[serde(default = "MpdConfig::default_volume_increment_fine")]
    pub volume_increment_fine: u32,
}

impl MpdConfig {
    fn default_address() -> String {
        "localhost:6600".to_string()
    }
    fn default_volume_increment() -> u32 {
        5
    }
    fn default_volume_increment_fine() -> u32 {
        1
    }
}

impl Default for MpdConfig {
    fn default() -> Self {
        Self {
            address: Self::default_address(),
            volume_increment: Self::default_volume_increment(),
            volume_increment_fine: Self::default_volume_increment_fine(),
        }
    }
}
