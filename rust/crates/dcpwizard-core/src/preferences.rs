use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub default_standard: String,
    pub default_resolution: String,
    pub default_frame_rate: u32,
    pub default_bitrate_mbps: u32,
    pub default_audio_channels: u32,
    pub default_output_dir: String,
    pub auto_verify: bool,
    pub auto_hash: bool,
    pub j2k_encoder: String,
    pub ffmpeg_path: String,
    pub asdcp_path: String,
    pub theme: String,
    pub recent_projects: Vec<String>,
    pub max_threads: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            default_standard: "SMPTE".into(),
            default_resolution: "2K".into(),
            default_frame_rate: 24,
            default_bitrate_mbps: 250,
            default_audio_channels: 6,
            default_output_dir: String::new(),
            auto_verify: true,
            auto_hash: true,
            j2k_encoder: "grk_compress".into(),
            ffmpeg_path: "ffmpeg".into(),
            asdcp_path: "asdcp-wrap".into(),
            theme: "dark".into(),
            recent_projects: Vec::new(),
            max_threads: num_cpus(),
        }
    }
}

/// Get the path to the preferences JSON file.
pub fn preferences_path() -> PathBuf {
    // Try XDG config dir, then HOME, then fallback
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg)
            .join("dcpwizard")
            .join("preferences.json");
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("dcpwizard")
            .join("preferences.json");
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata)
            .join("dcpwizard")
            .join("preferences.json");
    }

    PathBuf::from("dcpwizard_preferences.json")
}

/// Load preferences from disk. Returns defaults if file doesn't exist.
pub fn load_preferences() -> Preferences {
    let path = preferences_path();

    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<Preferences>(&content) {
            Ok(prefs) => prefs,
            Err(e) => {
                tracing::warn!(
                    "Failed to parse preferences at {}, using defaults: {e}",
                    path.display()
                );
                Preferences::default()
            }
        },
        Err(_) => Preferences::default(),
    }
}

/// Save preferences to disk.
pub fn save_preferences(prefs: &Preferences) -> i32 {
    let path = preferences_path();

    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent) {
            tracing::error!(
                "Failed to create preferences directory {}: {e}",
                parent.display()
            );
            return -1;
        }

    match serde_json::to_string_pretty(prefs) {
        Ok(json) => match std::fs::write(&path, json) {
            Ok(()) => {
                tracing::info!("Saved preferences to {}", path.display());
                0
            }
            Err(e) => {
                tracing::error!("Failed to write preferences: {e}");
                -1
            }
        },
        Err(e) => {
            tracing::error!("Failed to serialize preferences: {e}");
            -1
        }
    }
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}
