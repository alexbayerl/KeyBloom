use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub num_leds: usize,
    pub transition_steps: usize,
    pub transition_delay_ms: u64,
    pub frame_delay_ms: u64,
    pub sample_step: usize,
    pub color_change_threshold: f32,
    pub brightness_factor: f32,
    pub saturation_factor: f32,
    pub debounce_duration_ms: u64,
    pub openrgb_host: String,
    pub openrgb_port: u16,
    pub device_name: String,
    pub monitor_index: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            num_leds: 5,
            transition_steps: 10,
            transition_delay_ms: 15,
            frame_delay_ms: 100,
            sample_step: 10,
            color_change_threshold: 0.05,
            brightness_factor: 5.0,
            saturation_factor: 4.0,
            debounce_duration_ms: 500,
            openrgb_host: "localhost".to_string(),
            openrgb_port: 6742,
            device_name: "G213".to_string(),
            monitor_index: 1,
        }
    }
}

impl Config {
    /// Return the path to the config file
    fn config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "AlexanderBayerl", "KeyBloom") {
            proj_dirs.config_dir().join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    /// Load configuration or create a default one
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => toml::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            let config = Self::default();
            // Save a new default config
            let _ = config.save();
            config
        }
    }

    /// Save configuration to disk
    pub fn save(&self) -> io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|err| {
            eprintln!("Failed to serialize configuration: {}", err);
            io::Error::new(io::ErrorKind::Other, "Serialization failed")
        })?;
        fs::write(&path, content).map_err(|err| {
            eprintln!("Failed to save configuration to {}: {}", path.display(), err);
            err
        })
    }
}
