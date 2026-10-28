// Filename-based channel auto-routing (dom#2134): given mono WAVs named with a
// channel suffix (Foo_L.wav, Foo.Lfe.wav, Foo-Rs.wav, ...), combine them
// into one interleaved multichannel WAV in the canonical DCP channel order.

use crate::audio_map::{AppliedAudioMap, parse_audio_map};
use postkit::audio_mix_matrix::{MixMatrix, mix_wav_files};
use std::path::{Path, PathBuf};

/// Canonical DCP (ST 428-12) channel index for a filename suffix, or None if the
/// suffix names no known channel. Case-insensitive.
pub fn channel_index(suffix: &str) -> Option<usize> {
    match suffix.to_lowercase().as_str() {
        "l" | "left" => Some(0),
        "r" | "right" => Some(1),
        "c" | "centre" | "center" => Some(2),
        "lfe" | "sub" => Some(3),
        "ls" => Some(4),
        "rs" => Some(5),
        "lc" => Some(6),
        "rc" => Some(7),
        "bsl" | "lss" => Some(8),
        "bsr" | "rss" => Some(9),
        "hi" => Some(14),
        "vi" | "vin" => Some(15),
        _ => None,
    }
}

const CHANNEL_SUFFIX_SEPARATORS: [char; 4] = ['_', '.', '-', ' '];

/// Split a channel file's stem at the last `_`, `.`, `-` or space into the
/// set's name and the channel token: "260810 DST_MIX_V2.3.2.C.wav" gives
/// ("260810 DST_MIX_V2.3.2", "C").
pub fn channel_stem(path: &Path) -> Option<(String, String)> {
    let stem = path.file_stem()?.to_str()?;
    let (prefix, suffix) = stem.rsplit_once(CHANNEL_SUFFIX_SEPARATORS)?;
    Some((prefix.to_string(), suffix.to_string()))
}

pub fn named_lane(path: &Path) -> Option<usize> {
    channel_stem(path).and_then(|(_, suffix)| channel_index(&suffix))
}

pub fn channel_file_lane(path: &Path) -> usize {
    named_lane(path).unwrap_or_else(|| channel_index("C").expect("C names a lane"))
}

// row n of a channel set's audio map is file n in this order
pub fn channel_files_in_set_order(paths: &[PathBuf]) -> Vec<(usize, PathBuf)> {
    let mut files: Vec<(usize, PathBuf)> = paths
        .iter()
        .map(|path| (channel_file_lane(path), path.clone()))
        .collect();
    files.sort_by(|(lane, path), (other_lane, other_path)| {
        lane.cmp(other_lane)
            .then_with(|| path.file_name().cmp(&other_path.file_name()))
    });
    files
}

pub fn channel_set_matrix(paths: &[PathBuf], audio_map: Option<&str>) -> Result<MixMatrix, String> {
    if paths.is_empty() {
        return Err("no channel WAVs to route".to_string());
    }
    let spec = match audio_map {
        Some(spec) => spec.to_string(),
        None => channel_files_in_set_order(paths)
            .iter()
            .enumerate()
            .map(|(row, (lane, _))| format!("{}:{}", row + 1, lane + 1))
            .collect::<Vec<_>>()
            .join(","),
    };
    parse_audio_map(&spec, paths.len())
}

pub fn mix_channel_set(
    paths: &[PathBuf],
    audio_map: Option<&str>,
    output: &Path,
) -> Result<AppliedAudioMap, String> {
    let matrix = channel_set_matrix(paths, audio_map)?;
    let inputs: Vec<PathBuf> = channel_files_in_set_order(paths)
        .into_iter()
        .map(|(_, path)| path)
        .collect();
    for path in &inputs {
        let channels = postkit::wav_io::channel_count(path)?;
        if channels != 1 {
            return Err(format!(
                "{}: channel files must be mono, got {channels} channels",
                path.display()
            ));
        }
    }
    let report = mix_wav_files(&matrix, &inputs, output)?;
    Ok(AppliedAudioMap {
        pure_routing: matrix.is_pure_routing(),
        report,
    })
}

pub fn channel_directory_wavs(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut wavs: Vec<PathBuf> = Vec::new();
    let rd = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for e in rd.filter_map(|e| e.ok()) {
        let path = e.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.to_lowercase())
                == Some("wav".to_string())
        {
            wavs.push(path);
        }
    }
    if wavs.is_empty() {
        return Err(format!("no channel WAVs found in {}", dir.display()));
    }
    Ok(wavs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

    const SR: u32 = 48000;
    const FULL_SCALE: i32 = 1 << 23;

    fn write_mono(path: &Path, value: i32, frames: usize) {
        let spec = WavSpec {
            channels: 1,
            sample_rate: SR,
            bits_per_sample: 24,
            sample_format: SampleFormat::Int,
        };
        let mut w = WavWriter::create(path, spec).unwrap();
        for _ in 0..frames {
            w.write_sample(value).unwrap();
        }
        w.finalize().unwrap();
    }

    fn first_frame(path: &Path) -> Vec<i32> {
        let mut reader = WavReader::open(path).unwrap();
        let channels = usize::from(reader.spec().channels);
        reader
            .samples::<i32>()
            .take(channels)
            .map(|sample| sample.unwrap())
            .collect()
    }

    #[test]
    fn a_named_lane_is_the_lane_and_any_other_name_is_centre() {
        let centre = channel_index("C").unwrap();
        assert_eq!(channel_file_lane(Path::new("mix_Lfe.wav")), 3);
        assert_eq!(
            channel_file_lane(Path::new("260810 DST_MIX_V2.3.2.Rs.wav")),
            5
        );
        assert_eq!(named_lane(Path::new("mix_dialogue.wav")), None);
        assert_eq!(channel_file_lane(Path::new("mix_dialogue.wav")), centre);
        assert_eq!(channel_file_lane(Path::new("dialogue.wav")), centre);
    }

    #[test]
    fn a_set_is_ordered_by_lane_then_file_name() {
        let ordered = channel_files_in_set_order(&[
            PathBuf::from("/a/mix_Rs.wav"),
            PathBuf::from("/a/mix_music.wav"),
            PathBuf::from("/b/mix_C.wav"),
            PathBuf::from("/a/mix_L.wav"),
            PathBuf::from("/a/mix_dialogue.wav"),
        ]);
        let names: Vec<(usize, &str)> = ordered
            .iter()
            .map(|(lane, path)| (*lane, path.file_name().unwrap().to_str().unwrap()))
            .collect();
        assert_eq!(
            names,
            [
                (0, "mix_L.wav"),
                (2, "mix_C.wav"),
                (2, "mix_dialogue.wav"),
                (2, "mix_music.wav"),
                (5, "mix_Rs.wav"),
            ]
        );
    }

    #[test]
    fn routes_five_one_in_dcp_order() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        // odd values so a lossy step would show
        let values = [
            ("L", FULL_SCALE / 10 + 1),
            ("R", FULL_SCALE / 5 + 3),
            ("C", FULL_SCALE / 4 + 5),
            ("Lfe", FULL_SCALE / 3 + 7),
            ("Ls", FULL_SCALE / 2 + 9),
            ("Rs", -(FULL_SCALE / 3) * 2 - 11),
        ];
        for (lane, value) in values {
            write_mono(&d.join(format!("mix_{lane}.wav")), value, 100);
        }

        let out = d.join("routed.wav");
        let applied = mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &out).unwrap();
        assert!(applied.pure_routing);
        assert_eq!(applied.report.clipped_samples, 0);
        assert_eq!(
            first_frame(&out),
            values.map(|(_, value)| value),
            "each sample lands on its lane unchanged"
        );
    }

    #[test]
    fn missing_lfe_leaves_silent_lane() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("m_L.wav"), FULL_SCALE / 10, 50);
        write_mono(&d.join("m_R.wav"), FULL_SCALE / 5, 50);
        write_mono(&d.join("m_Ls.wav"), FULL_SCALE / 2, 50);
        write_mono(&d.join("m_Rs.wav"), FULL_SCALE / 3, 50);
        let out = d.join("o.wav");
        mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &out).unwrap();
        let samples = first_frame(&out);
        assert_eq!(samples.len(), 6);
        assert_eq!(samples[2], 0, "C silent");
        assert_eq!(samples[3], 0, "LFE silent");
        assert_eq!(samples[4], FULL_SCALE / 2, "Ls present");
    }

    #[test]
    fn dotted_suffix_routes_and_widens_to_five_one() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("260810 DST_MIX_V2.3.2.L.wav"), FULL_SCALE / 10, 20);
        write_mono(&d.join("260810 DST_MIX_V2.3.2.LFE.wav"), FULL_SCALE / 3, 20);
        assert_eq!(
            channel_stem(&d.join("260810 DST_MIX_V2.3.2.LFE.wav")),
            Some(("260810 DST_MIX_V2.3.2".to_string(), "LFE".to_string()))
        );
        let out = d.join("routed.wav");
        mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &out).unwrap();
        assert_eq!(
            first_frame(&out),
            [FULL_SCALE / 10, 0, 0, FULL_SCALE / 3, 0, 0]
        );
    }

    #[test]
    fn file_list_routes_like_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("mix-R.wav"), FULL_SCALE / 5, 30);
        write_mono(&d.join("mix L.wav"), FULL_SCALE / 10, 30);
        write_mono(&d.join("mix_C.wav"), FULL_SCALE / 4, 30);
        let from_directory = d.join("from_directory.wav");
        mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &from_directory).unwrap();
        let listed = [
            d.join("mix_C.wav"),
            d.join("mix L.wav"),
            d.join("mix-R.wav"),
        ];
        assert_eq!(
            channel_set_matrix(&listed, None).unwrap().output_channels(),
            6
        );
        let from_list = d.join("from_list.wav");
        mix_channel_set(&listed, None, &from_list).unwrap();
        assert_eq!(
            std::fs::read(&from_directory).unwrap(),
            std::fs::read(&from_list).unwrap()
        );
    }

    #[test]
    fn two_files_on_centre_are_summed() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("mix_L.wav"), FULL_SCALE / 10, 30);
        write_mono(&d.join("mix_dialogue.wav"), FULL_SCALE / 4, 30);
        write_mono(&d.join("mix_music.wav"), FULL_SCALE / 8, 30);
        let out = d.join("o.wav");
        let applied = mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &out).unwrap();
        assert!(!applied.pure_routing);
        assert_eq!(
            first_frame(&out),
            [FULL_SCALE / 10, 0, FULL_SCALE / 4 + FULL_SCALE / 8, 0, 0, 0]
        );
    }

    #[test]
    fn a_typed_map_moves_an_unnamed_file_off_centre() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("mix_L.wav"), FULL_SCALE / 10, 30);
        write_mono(&d.join("mix_R.wav"), FULL_SCALE / 5, 30);
        write_mono(&d.join("mix_boom.wav"), FULL_SCALE / 3, 30);
        let out = d.join("o.wav");
        // set order is L, R, then the unnamed file at centre
        let applied = mix_channel_set(
            &channel_directory_wavs(d).unwrap(),
            Some("1:L,2:R,3:LFE"),
            &out,
        )
        .unwrap();
        assert!(applied.pure_routing);
        assert_eq!(
            first_frame(&out),
            [FULL_SCALE / 10, FULL_SCALE / 5, 0, FULL_SCALE / 3, 0, 0]
        );
    }

    #[test]
    fn rejects_non_mono() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let spec = WavSpec {
            channels: 2,
            sample_rate: SR,
            bits_per_sample: 24,
            sample_format: SampleFormat::Int,
        };
        let mut w = WavWriter::create(d.join("m_L.wav"), spec).unwrap();
        for _ in 0..10 {
            w.write_sample(0i32).unwrap();
            w.write_sample(0i32).unwrap();
        }
        w.finalize().unwrap();
        write_mono(&d.join("m_R.wav"), 0, 10);
        let error = mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &d.join("o.wav"))
            .unwrap_err();
        assert!(error.contains("must be mono"), "{error}");
    }

    #[test]
    fn rejects_a_second_sample_rate() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("m_L.wav"), 0, 10);
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 24,
            sample_format: SampleFormat::Int,
        };
        WavWriter::create(d.join("m_R.wav"), spec)
            .unwrap()
            .finalize()
            .unwrap();
        let error = mix_channel_set(&channel_directory_wavs(d).unwrap(), None, &d.join("o.wav"))
            .unwrap_err();
        assert!(error.contains("sample rate"), "{error}");
    }
}
