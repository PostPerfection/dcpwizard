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
            _ => None,
        }
    }
}

/// Rates that require a 2K container: the DCI HFR addendum / SMPTE ST 428-11:2013
/// only add 48/50/60 for 2K. 4K exhibition tops out at 30 fps.
const HFR_RATES_2K_ONLY: [u32; 3] = [48, 50, 60];

// the addendum's other 2K rates, which ST 429-2 §8.1 gives no composition edit rate value for
const RATES_ABOVE_THE_ST_429_2_CAP: [u32; 3] = [96, 100, 120];

const NTSC_RATE_DENOMINATOR: u32 = 1_001;
const NTSC_RATE_NUMERATOR_SCALE: u32 = 1_000;

/// Convert a probed source rate into the integer DCP edit rate. An N*1000/1001
/// source whose N is a DCP rate is deliberately played at N fps and requires
/// matching audio pull-up.
pub fn source_rate_to_dcp(fps_num: u32, fps_den: u32) -> (u32, bool) {
    let pulled_up_rate = fps_num / NTSC_RATE_NUMERATOR_SCALE;
    let is_ntsc_rate = fps_den == NTSC_RATE_DENOMINATOR
        && fps_num.is_multiple_of(NTSC_RATE_NUMERATOR_SCALE)
        && FrameRate::from_u32(pulled_up_rate).is_some();
    if is_ntsc_rate {
        return (pulled_up_rate, true);
    }
    (fps_num / fps_den.max(1), false)
}

/// How a source is conformed to the DCP edit rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceConform {
    /// Read the picture as if it ran at this rate, ignoring the source's own
    /// timestamps, so every frame is packaged once. None reads it as it is.
    pub read_source_at: Option<postkit::encode::FrameRate>,
    /// Whether the sound has to be pulled up by the same 1000/1001 to stay in
    /// sync with a picture that now plays 0.1% faster.
    pub audio_pull_up: bool,
}

/// What it takes to package a source at the requested DCP edit rate: only an
/// N*1000/1001 source at N fps needs anything, and it plays 0.1% faster with
/// the sound pulled up to match, the way DCP-o-matic conforms it.
pub fn conform_source_to_dcp(fps_num: u32, fps_den: u32, dcp_fps: u32) -> SourceConform {
    let (source_dcp_fps, needs_pull_up) = source_rate_to_dcp(fps_num, fps_den);
    if !needs_pull_up || dcp_fps != source_dcp_fps {
        return SourceConform::default();
    }
    SourceConform {
        read_source_at: Some(postkit::encode::FrameRate::whole(dcp_fps)),
        audio_pull_up: true,
    }
}

/// Speed the sound up by 1000/1001 to match an N*1000/1001 picture packaged at N fps:
/// reinterpret the samples at 48048 Hz, then resample back to the 48 kHz a DCP
/// sound track carries.
pub fn audio_pull_up(input: &std::path::Path, output: &std::path::Path) -> Result<(), String> {
    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input)
        .arg("-af")
        .arg("asetrate=48048,aresample=48000")
        // the wrap wants the same PCM it would have had, only faster
        .args(["-c:a", "pcm_s24le", "-rf64", "auto"])
        .arg(output)
        .output()
        .map_err(|error| format!("failed to run ffmpeg for audio pull-up: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "1000/1001 audio pull-up failed: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}

/// Check whether a frame rate is valid for the given DCP standard, ignoring the
/// resolution constraint. Use [`validate_fps_resolution`] to also reject an
/// illegal fps/resolution combination.
///
/// SMPTE DCP rates: 24, 25, 30, 48, 50, 60.
/// Interop rates: 24, 25, 30, 48 (no HFR).
pub fn is_valid_frame_rate(fps: u32, smpte: bool) -> bool {
    if smpte {
        matches!(fps, 24 | 25 | 30 | 48 | 50 | 60)
    } else {
        matches!(fps, 24 | 25 | 30 | 48)
    }
}

/// Reject an illegal frame rate or fps/resolution combination before any encode
/// or wrap runs. `is_4k` is the picture container class (4096-wide family).
///
/// 4K is limited to 24/25/30 fps; 48 fps and up require a 2K container per the
/// DCI HFR addendum. 48/50/60 are SMPTE-only.
pub fn validate_fps_resolution(fps: u32, is_4k: bool, smpte: bool) -> Result<(), String> {
    if RATES_ABOVE_THE_ST_429_2_CAP.contains(&fps) {
        return Err(format!(
            "frame rate {fps} fps cannot be packaged: ST 429-2 stops a 2K composition edit rate at 60 fps"
        ));
    }
    if !is_valid_frame_rate(fps, smpte) {
        let legal = if smpte {
            "24, 25, 30, 48, 50, 60"
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
        for fps in [24, 25, 30, 48, 50, 60] {
            assert!(
                is_valid_frame_rate(fps, true),
                "{fps} should be legal SMPTE"
            );
        }
        assert!(!is_valid_frame_rate(23, true));
        assert!(!is_valid_frame_rate(50, false), "interop has no HFR");
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
        for fps in [48, 50, 60] {
            assert!(
                validate_fps_resolution(fps, true, true).is_err(),
                "4K@{fps} must be rejected"
            );
        }
    }

    #[test]
    fn two_k_allows_hfr() {
        for fps in [48, 50, 60] {
            assert!(validate_fps_resolution(fps, false, true).is_ok());
        }
    }

    #[test]
    fn the_addendum_rates_above_60_name_st_429_2() {
        for fps in [96, 100, 120] {
            assert!(!is_valid_frame_rate(fps, true), "{fps} is no SMPTE rate");
            let refusal = validate_fps_resolution(fps, false, true).unwrap_err();
            assert!(
                refusal.contains(&format!("{fps} fps")) && refusal.contains("ST 429-2"),
                "the refusal names the rate and the standard: {refusal}"
            );
        }
    }

    #[test]
    fn illegal_rate_rejected_at_any_resolution() {
        assert!(validate_fps_resolution(23, false, true).is_err());
        assert!(validate_fps_resolution(50, false, false).is_err());
    }

    #[test]
    fn ntsc_rates_map_to_their_dcp_rate_with_audio_pull_up() {
        assert_eq!(source_rate_to_dcp(24_000, 1_001), (24, true));
        assert_eq!(source_rate_to_dcp(30_000, 1_001), (30, true));
        assert_eq!(source_rate_to_dcp(48_000, 1_001), (48, true));
        assert_eq!(source_rate_to_dcp(60_000, 1_001), (60, true));
        assert_eq!(source_rate_to_dcp(24, 1), (24, false));
        assert_eq!(source_rate_to_dcp(60, 1), (60, false));
        assert_eq!(source_rate_to_dcp(120_000, 1_001), (119, false));
    }

    #[test]
    fn only_an_ntsc_rate_at_its_dcp_rate_is_conformed() {
        for (fps_num, dcp_fps) in [(24_000, 24), (30_000, 30), (48_000, 48), (60_000, 60)] {
            let conform = conform_source_to_dcp(fps_num, 1_001, dcp_fps);
            assert_eq!(
                conform.read_source_at,
                Some(postkit::encode::FrameRate::whole(dcp_fps))
            );
            assert!(conform.audio_pull_up, "{fps_num}/1001 at {dcp_fps} fps");
        }

        for (fps_num, fps_den, dcp_fps) in [
            (24_000, 1_001, 25),
            (24_000, 1_001, 48),
            (24, 1, 24),
            (25, 1, 24),
            (30_000, 1_001, 24),
            (60_000, 1_001, 30),
        ] {
            assert_eq!(
                conform_source_to_dcp(fps_num, fps_den, dcp_fps),
                SourceConform::default(),
                "{fps_num}/{fps_den} at {dcp_fps} fps needs no conform"
            );
        }
    }

    #[test]
    fn roundtrip_u32() {
        for fps in [24, 25, 30, 48, 50, 60] {
            assert_eq!(FrameRate::from_u32(fps).unwrap().as_u32(), fps);
        }
        assert!(FrameRate::from_u32(23).is_none());
        assert!(FrameRate::from_u32(120).is_none());
    }
}
