//! Configuration handling for specks

use serde::{Deserialize, Serialize};

/// Specks configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Specks-specific settings
    #[serde(default)]
    pub specks: SpecksConfig,
}

/// Core specks settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecksConfig {
    /// Validation strictness level
    #[serde(default = "default_validation_level")]
    pub validation_level: String,

    /// Include info-level messages in validation output
    #[serde(default)]
    pub show_info: bool,
}

fn default_validation_level() -> String {
    "normal".to_string()
}

impl Default for SpecksConfig {
    fn default() -> Self {
        Self {
            validation_level: default_validation_level(),
            show_info: false,
        }
    }
}
