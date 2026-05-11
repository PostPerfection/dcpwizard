use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Reel split mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ReelSplitMode {
    #[default]
    None,
    ByDuration,
    BySize,
}

/// Reel configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReelConfig {
    pub split_mode: ReelSplitMode,
    pub split_duration_minutes: u32,
    pub split_size_bytes: u64,
}

/// Reel information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReelInfo {
    pub id: String,
    pub frame_start: u64,
    pub frame_end: u64,
    pub picture_mxf: PathBuf,
    pub sound_mxf: PathBuf,
    pub subtitle_mxf: PathBuf,
}

/// Plan reel splits for the given duration/size.
pub fn plan_reels(total_frames: u64, frame_rate: u32, config: &ReelConfig) -> Vec<ReelInfo> {
    match config.split_mode {
        ReelSplitMode::None => {
            vec![ReelInfo {
                id: uuid::Uuid::new_v4().to_string(),
                frame_start: 0,
                frame_end: total_frames,
                ..Default::default()
            }]
        }
        ReelSplitMode::ByDuration => {
            let frames_per_reel = (config.split_duration_minutes as u64) * 60 * (frame_rate as u64);
            if frames_per_reel == 0 {
                return vec![ReelInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    frame_start: 0,
                    frame_end: total_frames,
                    ..Default::default()
                }];
            }

            let mut reels = Vec::new();
            let mut start = 0;
            while start < total_frames {
                let end = (start + frames_per_reel).min(total_frames);
                reels.push(ReelInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    frame_start: start,
                    frame_end: end,
                    ..Default::default()
                });
                start = end;
            }
            reels
        }
        ReelSplitMode::BySize => {
            // Estimate: assume constant bitrate, calculate frames per reel
            if config.split_size_bytes == 0 || frame_rate == 0 {
                return vec![ReelInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    frame_start: 0,
                    frame_end: total_frames,
                    ..Default::default()
                }];
            }

            // Rough estimate: 250 Mbps = ~31.25 MB/s for 2K J2K
            let bytes_per_frame = 250_000_000u64 / 8 / (frame_rate as u64);
            let frames_per_reel = config.split_size_bytes / bytes_per_frame.max(1);

            let mut reels = Vec::new();
            let mut start = 0;
            while start < total_frames {
                let end = (start + frames_per_reel).min(total_frames);
                reels.push(ReelInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    frame_start: start,
                    frame_end: end,
                    ..Default::default()
                });
                start = end;
            }
            reels
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_reels_none() {
        let config = ReelConfig::default();
        let reels = plan_reels(48000, 24, &config);
        assert_eq!(reels.len(), 1);
        assert_eq!(reels[0].frame_start, 0);
        assert_eq!(reels[0].frame_end, 48000);
    }

    #[test]
    fn test_plan_reels_by_duration() {
        let config = ReelConfig {
            split_mode: ReelSplitMode::ByDuration,
            split_duration_minutes: 20,
            ..Default::default()
        };
        // 48000 frames at 24fps = 2000 seconds = 33.3 minutes → 2 reels
        let reels = plan_reels(48000, 24, &config);
        assert_eq!(reels.len(), 2);
    }
}
