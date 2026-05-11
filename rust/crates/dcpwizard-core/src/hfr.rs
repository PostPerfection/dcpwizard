use serde::{Deserialize, Serialize};

/// DCI/SMPTE frame rate values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRate {
    Fps24,
    Fps25,
    Fps30,
    Fps48,
    Fps60,
    Fps96,
    Fps120,
}

impl FrameRate {
    pub fn as_u32(&self) -> u32 {
        match self {
            FrameRate::Fps24 => 24,
            FrameRate::Fps25 => 25,
            FrameRate::Fps30 => 30,
            FrameRate::Fps48 => 48,
            FrameRate::Fps60 => 60,
            FrameRate::Fps96 => 96,
            FrameRate::Fps120 => 120,
        }
    }

    pub fn edit_rate(&self) -> (u32, u32) {
        (self.as_u32(), 1)
    }

    pub fn from_u32(fps: u32) -> Option<Self> {
        match fps {
            24 => Some(FrameRate::Fps24),
            25 => Some(FrameRate::Fps25),
            30 => Some(FrameRate::Fps30),
            48 => Some(FrameRate::Fps48),
            60 => Some(FrameRate::Fps60),
            96 => Some(FrameRate::Fps96),
            120 => Some(FrameRate::Fps120),
            _ => None,
        }
    }
}

/// Check whether a frame rate is valid for the given DCP standard.
///
/// SMPTE DCI frame rates: 24, 25, 30, 48, 60, 96, 120
/// Interop frame rates: 24, 25, 30, 48
pub fn is_valid_frame_rate(fps: u32, smpte: bool) -> bool {
    if smpte {
        matches!(fps, 24 | 25 | 30 | 48 | 60 | 96 | 120)
    } else {
        // Interop only supports 24, 25, 30, 48
        matches!(fps, 24 | 25 | 30 | 48)
    }
}

/// Return all supported frame rates for the given standard.
pub fn supported_frame_rates(smpte: bool) -> Vec<FrameRate> {
    if smpte {
        vec![
            FrameRate::Fps24,
            FrameRate::Fps25,
            FrameRate::Fps30,
            FrameRate::Fps48,
            FrameRate::Fps60,
            FrameRate::Fps96,
            FrameRate::Fps120,
        ]
    } else {
        vec![
            FrameRate::Fps24,
            FrameRate::Fps25,
            FrameRate::Fps30,
            FrameRate::Fps48,
        ]
    }
}
