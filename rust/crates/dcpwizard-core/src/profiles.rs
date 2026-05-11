use serde::{Deserialize, Serialize};

/// A DCP creation profile with preset settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub standard: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub frame_rate: u32,
    pub bitrate_mbps: u32,
    pub audio_channels: u32,
    pub audio_sample_rate: u32,
    pub content_kind: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "cinema_2k".into(),
            description: "Standard 2K cinema DCP".into(),
            standard: "SMPTE".into(),
            resolution_width: 2048,
            resolution_height: 1080,
            frame_rate: 24,
            bitrate_mbps: 250,
            audio_channels: 6,
            audio_sample_rate: 48000,
            content_kind: "feature".into(),
        }
    }
}

/// Get a profile by name.
pub fn get_profile(name: &str) -> Option<Profile> {
    let profiles = all_profiles();
    profiles.into_iter().find(|p| p.name == name)
}

/// Return all built-in profiles.
pub fn all_profiles() -> Vec<Profile> {
    vec![
        Profile {
            name: "cinema_2k".into(),
            description: "Standard 2K cinema DCP (Flat/Scope)".into(),
            standard: "SMPTE".into(),
            resolution_width: 2048,
            resolution_height: 1080,
            frame_rate: 24,
            bitrate_mbps: 250,
            audio_channels: 6,
            audio_sample_rate: 48000,
            content_kind: "feature".into(),
        },
        Profile {
            name: "cinema_4k".into(),
            description: "4K cinema DCP".into(),
            standard: "SMPTE".into(),
            resolution_width: 4096,
            resolution_height: 2160,
            frame_rate: 24,
            bitrate_mbps: 500,
            audio_channels: 6,
            audio_sample_rate: 48000,
            content_kind: "feature".into(),
        },
        Profile {
            name: "trailer".into(),
            description: "Cinema trailer DCP".into(),
            standard: "SMPTE".into(),
            resolution_width: 2048,
            resolution_height: 858,
            frame_rate: 24,
            bitrate_mbps: 250,
            audio_channels: 6,
            audio_sample_rate: 48000,
            content_kind: "trailer".into(),
        },
        Profile {
            name: "advertisement".into(),
            description: "Cinema advertisement DCP".into(),
            standard: "SMPTE".into(),
            resolution_width: 2048,
            resolution_height: 1080,
            frame_rate: 24,
            bitrate_mbps: 200,
            audio_channels: 2,
            audio_sample_rate: 48000,
            content_kind: "advertisement".into(),
        },
    ]
}
