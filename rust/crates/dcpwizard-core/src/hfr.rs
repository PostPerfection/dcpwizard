use serde::{Deserialize, Serialize};

/// DCI/SMPTE frame rate values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRate {
    Fps24,
    Fps25,
    Fps30,
    Fps48,
    Fps50,
    Fps60,
    Fps96,
    Fps100,
    Fps120,
}

impl FrameRate {
    pub fn as_u32(&self) -> u32 {
        match self {
            FrameRate::Fps24 => 24,
            FrameRate::Fps25 => 25,
            FrameRate::Fps30 => 30,
            FrameRate::Fps48 => 48,
            FrameRate::Fps50 => 50,
            FrameRate::Fps60 => 60,
            FrameRate::Fps96 => 96,
            FrameRate::Fps100 => 100,
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
            50 => Some(FrameRate::Fps50),
            60 => Some(FrameRate::Fps60),
            96 => Some(FrameRate::Fps96),
            100 => Some(FrameRate::Fps100),
            120 => Some(FrameRate::Fps120),
            _ => None,
        }
    }
}

/// Rates that require a 2K container: the DCI HFR addendum / SMPTE ST 428-11:2013
/// only add 48/50/60 and 96/100/120 for 2K. 4K exhibition tops out at 30 fps.
const HFR_RATES_2K_ONLY: [u32; 6] = [48, 50, 60, 96, 100, 120];

/// Convert a probed source rate into the integer DCP edit rate. A 24000/1001
/// source is deliberately played at 24 fps and requires matching audio pull-up.
pub fn source_rate_to_dcp(fps_num: u32, fps_den: u32) -> (u32, bool) {
    if fps_num == 24_000 && fps_den == 1_001 {
        return (24, true);
    }
    (fps_num / fps_den.max(1), false)
}

/// Check whether a frame rate is valid for the given DCP standard, ignoring the
/// resolution constraint. Use [`validate_fps_resolution`] to also reject an
/// illegal fps/resolution combination.
///
/// SMPTE DCP rates: 24, 25, 30, 48, 50, 60, 96, 100, 120.
/// Interop rates: 24, 25, 30, 48 (no HFR).
pub fn is_valid_frame_rate(fps: u32, smpte: bool) -> bool {
    if smpte {
        matches!(fps, 24 | 25 | 30 | 48 | 50 | 60 | 96 | 100 | 120)
    } else {
        matches!(fps, 24 | 25 | 30 | 48)
    }
}

/// Reject an illegal frame rate or fps/resolution combination before any encode
/// or wrap runs. `is_4k` is the picture container class (4096-wide family).
///
/// 4K is limited to 24/25/30 fps; 48 fps and up require a 2K container per the
/// DCI HFR addendum. HFR rates (96/100/120) are SMPTE-only.
pub fn validate_fps_resolution(fps: u32, is_4k: bool, smpte: bool) -> Result<(), String> {
    if !is_valid_frame_rate(fps, smpte) {
        let legal = if smpte {
            "24, 25, 30, 48, 50, 60, 96, 100, 120"
        } else {
            "24, 25, 30, 48"
        };
        return Err(format!(
            "frame rate {fps} fps is not a legal {} DCP rate; legal rates: {legal}",
            if smpte { "SMPTE" } else { "Interop" }
        ));
    }
    if is_4k && HFR_RATES_2K_ONLY.contains(&fps) {
        return Err(format!(
            "4K DCP is limited to 24/25/30 fps; {fps} fps requires a 2K container"
        ));
    }
    Ok(())
}

/// Return all supported frame rates for the given standard.
pub fn supported_frame_rates(smpte: bool) -> Vec<FrameRate> {
    if smpte {
        vec![
            FrameRate::Fps24,
            FrameRate::Fps25,
            FrameRate::Fps30,
            FrameRate::Fps48,
            FrameRate::Fps50,
            FrameRate::Fps60,
            FrameRate::Fps96,
            FrameRate::Fps100,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smpte_accepts_full_set_including_hfr() {
        for fps in [24, 25, 30, 48, 50, 60, 96, 100, 120] {
            assert!(
                is_valid_frame_rate(fps, true),
                "{fps} should be legal SMPTE"
            );
        }
        assert!(!is_valid_frame_rate(23, true));
        assert!(!is_valid_frame_rate(120, false), "interop has no HFR");
    }

    #[test]
    fn interop_rejects_hfr() {
        assert!(is_valid_frame_rate(48, false));
        assert!(!is_valid_frame_rate(50, false));
        assert!(!is_valid_frame_rate(60, false));
    }

    #[test]
    fn four_k_rejects_rates_above_30() {
        assert!(validate_fps_resolution(30, true, true).is_ok());
        for fps in [48, 50, 60, 96, 100, 120] {
            assert!(
                validate_fps_resolution(fps, true, true).is_err(),
                "4K@{fps} must be rejected"
            );
        }
    }

    #[test]
    fn two_k_allows_hfr() {
        for fps in [48, 50, 60, 96, 100, 120] {
            assert!(validate_fps_resolution(fps, false, true).is_ok());
        }
    }

    #[test]
    fn illegal_rate_rejected_at_any_resolution() {
        assert!(validate_fps_resolution(23, false, true).is_err());
        assert!(validate_fps_resolution(120, false, false).is_err());
    }

    #[test]
    fn ntsc_film_maps_to_24_with_audio_pull_up() {
        assert_eq!(source_rate_to_dcp(24_000, 1_001), (24, true));
        assert_eq!(source_rate_to_dcp(24, 1), (24, false));
    }

    #[test]
    fn roundtrip_u32() {
        for fps in [24, 25, 30, 48, 50, 60, 96, 100, 120] {
            assert_eq!(FrameRate::from_u32(fps).unwrap().as_u32(), fps);
        }
        assert!(FrameRate::from_u32(23).is_none());
    }
}
