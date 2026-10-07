//! The audio mix matrix as `create` spells it: postkit's `IN:OUT[@GAIN]`
//! grammar, with DCP lane names accepted wherever an output channel number is.
//!
//! `1:L,2:R,1:C@-6` reads the same to the CLI flag and to the GUI matrix widget,
//! because both come through here.

use std::path::Path;

use postkit::audio_mix_matrix::{
    LaneVocabulary, MixMatrix, MixReport, mix_wav_files, parse_named_audio_map,
};

/// The DCP lanes a map may name, in the order [`crate::audio_route::channel_index`]
/// puts them. HI and VI sit at lanes 14 and 15, so naming one widens the output
/// to the full 16-channel track.
pub const DCP_LANE_NAMES: [&str; 12] = [
    "L", "R", "C", "LFE", "Ls", "Rs", "Lc", "Rc", "BsL", "BsR", "HI", "VI",
];

/// Channel counts the sound wrapper accepts. Nothing in DCP has three channels,
/// so a map that reaches the centre lane leaves the surrounds silent instead of
/// writing a track asdcplib refuses.
const DCP_SOUND_LAYOUTS: [usize; 4] = [2, 6, 8, 16];

/// What an applied map did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedAudioMap {
    pub report: MixReport,
    /// Every output takes at most one input at unity gain, so the samples moved
    /// without their values being touched.
    pub pure_routing: bool,
}

/// Parse a map against a source of `input_channels` channels.
///
/// The grammar is postkit's, with one addition: an output may be a DCP lane name
/// (case-insensitive) instead of a channel number. The output holds every lane
/// an entry lands on, widened to the smallest sound layout a DCP is wrapped at,
/// with the lanes nothing routed to silent.
pub fn parse_audio_map(spec: &str, input_channels: usize) -> Result<MixMatrix, String> {
    parse_named_audio_map(spec, input_channels, &lane_vocabulary())
}

fn lane_vocabulary() -> LaneVocabulary {
    let lane_count = *DCP_SOUND_LAYOUTS.last().expect("the layouts are fixed");
    let mut lane_names = vec![Vec::new(); lane_count];
    for name in DCP_LANE_NAMES {
        let lane = crate::audio_route::channel_index(name).expect("every listed lane has an index");
        lane_names[lane].push(name.to_string());
    }
    LaneVocabulary {
        lane_names,
        output_channel_count: dcp_sound_channel_count,
    }
}

/// The sound layout that holds `highest_lane`. A lane past every layout is left
/// alone, so the wrap refuses it by name rather than this rounding it away.
fn dcp_sound_channel_count(highest_lane: usize) -> usize {
    DCP_SOUND_LAYOUTS
        .into_iter()
        .find(|layout| *layout >= highest_lane)
        .unwrap_or(highest_lane)
}

/// Mix `input` through the map into `output`.
pub fn apply_audio_map(spec: &str, input: &Path, output: &Path) -> Result<AppliedAudioMap, String> {
    let matrix = parse_audio_map(spec, postkit::wav_io::channel_count(input)?)?;
    let report = mix_wav_files(&matrix, std::slice::from_ref(&input.to_path_buf()), output)?;
    Ok(AppliedAudioMap {
        pure_routing: matrix.is_pure_routing(),
        report,
    })
}

/// A map typed against a channel set, where input `n` is the set's `n`th file
/// in lane order, rewritten against the routed WAV, where that file sits at its
/// lane. `routed` is the set as [`crate::audio_route::channel_lanes`] gives it.
pub fn channel_set_audio_map(
    spec: &str,
    routed: &[(usize, std::path::PathBuf)],
) -> Result<String, String> {
    let entries = spec
        .split(',')
        .map(|entry| {
            let entry = entry.trim();
            let (row, rest) = entry
                .split_once(':')
                .ok_or_else(|| format!("audio map entry '{entry}' names no output"))?;
            let lane = row
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|row| row.checked_sub(1))
                .and_then(|index| routed.get(index))
                .ok_or_else(|| {
                    format!(
                        "audio map entry '{entry}': the channel set has {} files",
                        routed.len()
                    )
                })?;
            Ok(format!("{}:{rest}", lane.0 + 1))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(entries.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_channel_set_map_reads_each_row_as_the_lane_its_file_routes_to() {
        // L, R, Ls, Rs: no C or LFE, so rows 3 and 4 are routed channels 5 and 6
        let routed: Vec<(usize, std::path::PathBuf)> = [0, 1, 4, 5]
            .into_iter()
            .map(|lane| (lane, std::path::PathBuf::from(format!("mix_{lane}.wav"))))
            .collect();
        assert_eq!(
            channel_set_audio_map("1:L,2:R,3:Ls@-3,4:Rs,3:C@-6", &routed).unwrap(),
            "1:L,2:R,5:Ls@-3,6:Rs,5:C@-6"
        );
        let error = channel_set_audio_map("5:L", &routed).unwrap_err();
        assert!(error.contains("has 4 files"), "{error}");
    }

    const STEREO: usize = 2;

    #[test]
    fn lane_names_resolve_to_the_canonical_channel_order() {
        let matrix = parse_audio_map("1:L,2:R,1:C@-6", STEREO).unwrap();
        assert_eq!(
            matrix.output_channels(),
            6,
            "the centre lane widens the track to 5.1, with the surrounds silent"
        );
        assert_eq!(matrix.gain_db(0, 0), Some(0.0));
        assert_eq!(matrix.gain_db(1, 1), Some(0.0));
        assert!((matrix.gain_db(0, 2).unwrap() + 6.0).abs() < 1e-9);
        assert_eq!(matrix.gain_db(1, 2), None);
    }

    #[test]
    fn a_lane_name_is_case_insensitive_and_matches_its_number() {
        let named = parse_audio_map("1:lfe", STEREO).unwrap();
        let numbered = parse_audio_map("1:4", STEREO).unwrap();
        assert_eq!(named, numbered);
        assert_eq!(named.output_channels(), 6);
    }

    #[test]
    fn the_accessibility_lanes_widen_the_track_to_sixteen_channels() {
        for spec in ["1:HI", "2:vi"] {
            assert_eq!(
                parse_audio_map(spec, STEREO).unwrap().output_channels(),
                16,
                "{spec} must reach the full 16-channel track"
            );
        }
    }

    #[test]
    fn a_plain_routing_is_bit_exact_and_a_gain_is_not() {
        assert!(
            parse_audio_map("1:L,2:R", STEREO)
                .unwrap()
                .is_pure_routing()
        );
        assert!(
            !parse_audio_map("1:L,2:L", STEREO)
                .unwrap()
                .is_pure_routing(),
            "two inputs summing into one lane is a mix"
        );
        assert!(!parse_audio_map("1:L@-6", STEREO).unwrap().is_pure_routing());
    }

    #[test]
    fn an_input_outside_the_source_is_refused() {
        let error = parse_audio_map("3:L", STEREO).unwrap_err();
        assert!(error.contains("input channel 3"), "{error}");
    }

    #[test]
    fn an_unknown_lane_name_lists_the_ones_that_exist() {
        let error = parse_audio_map("1:Surround", STEREO).unwrap_err();
        assert!(error.contains("Surround"), "{error}");
        assert!(error.contains("LFE"), "{error}");
    }

    #[test]
    fn a_malformed_map_fails_loud() {
        assert!(parse_audio_map("", STEREO).unwrap_err().contains("empty"));
        assert!(
            parse_audio_map("1:L,,2:R", STEREO)
                .unwrap_err()
                .contains("empty entry")
        );
        assert!(
            parse_audio_map("1-L", STEREO)
                .unwrap_err()
                .contains("IN:OUT")
        );
        assert!(
            parse_audio_map("1:L@loud", STEREO)
                .unwrap_err()
                .contains("gain")
        );
        assert!(
            parse_audio_map("1:0", STEREO)
                .unwrap_err()
                .contains("count from")
        );
        assert!(
            parse_audio_map("1:L,1:L", STEREO)
                .unwrap_err()
                .contains("twice")
        );
    }
}
