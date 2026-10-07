// Filename-based channel auto-routing (dom#2134): given mono WAVs named with a
// channel suffix (Foo_L.wav, Foo.Lfe.wav, Foo-Rs.wav, ...), combine them
// into one interleaved multichannel WAV in the canonical DCP channel order.

use hound::WavSpec;
use postkit::wav_io::{read_interleaved, write_interleaved};
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

fn suffix_of(path: &Path) -> Option<String> {
    channel_stem(path).map(|(_, suffix)| suffix)
}

/// Route every `*.wav` in `dir` to its channel lane and write one interleaved
/// multichannel WAV to `output`. Every file must carry a recognized channel
/// suffix, the rest is [`route_files`]. Returns `output`.
pub fn route_directory(dir: &Path, output: &Path) -> Result<PathBuf, String> {
    route_files(&channel_files(dir)?, output)
}

/// Write the mono files, each at its lane, as one interleaved multichannel WAV
/// to `output`. Every file must be mono and share sample rate / bit depth /
/// format, anything else fails loud. The output channel count is the highest
/// routed lane + 1, with unused lanes silent. Returns `output`.
pub fn route_files(entries: &[(usize, PathBuf)], output: &Path) -> Result<PathBuf, String> {
    if entries.is_empty() {
        return Err("no channel WAVs to route".to_string());
    }

    // read every channel, enforcing mono and a shared format.
    let mut spec: Option<WavSpec> = None;
    let mut lanes: Vec<(usize, Vec<f32>)> = Vec::new();
    for (idx, path) in entries {
        let (s, samples) =
            read_interleaved(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if s.channels != 1 {
            return Err(format!(
                "{}: channel files must be mono, got {} channels",
                path.display(),
                s.channels
            ));
        }
        match spec {
            None => spec = Some(s),
            Some(first) => {
                if first.sample_rate != s.sample_rate
                    || first.bits_per_sample != s.bits_per_sample
                    || first.sample_format != s.sample_format
                {
                    return Err(format!(
                        "{}: format {:?} differs from the first channel {:?}",
                        path.display(),
                        s,
                        first
                    ));
                }
                if lanes.iter().any(|(i, _)| i == idx) {
                    return Err(format!(
                        "{}: channel index {idx} routed twice",
                        path.display()
                    ));
                }
            }
        }
        lanes.push((*idx, samples));
    }

    let spec = spec.expect("entries non-empty");
    let channels = lanes.iter().map(|(i, _)| i + 1).max().unwrap();
    let frames = lanes.iter().map(|(_, s)| s.len()).max().unwrap();
    let mut interleaved = vec![0.0f32; frames * channels];
    for (idx, samples) in &lanes {
        for (f, &v) in samples.iter().enumerate() {
            interleaved[f * channels + idx] = v;
        }
    }

    write_interleaved(
        output,
        WavSpec {
            channels: channels as u16,
            ..spec
        },
        &interleaved,
    )
    .map_err(|e| format!("writing {}: {e}", output.display()))?;
    Ok(output.to_path_buf())
}

pub fn routed_channel_count(dir: &Path) -> Result<usize, String> {
    Ok(routed_channel_count_of(&channel_files(dir)?))
}

/// Highest routed lane + 1.
pub fn routed_channel_count_of(files: &[(usize, PathBuf)]) -> usize {
    files.iter().map(|(index, _)| index + 1).max().unwrap_or(0)
}

// every *.wav in dir with its lane, in lane order
fn channel_files(dir: &Path) -> Result<Vec<(usize, PathBuf)>, String> {
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
    channel_lanes(&wavs)
}

/// Each file with the lane its channel suffix names, in lane order. A file
/// with no suffix or an unknown one fails loud.
pub fn channel_lanes(paths: &[PathBuf]) -> Result<Vec<(usize, PathBuf)>, String> {
    let mut entries = paths
        .iter()
        .map(|path| {
            let suffix = suffix_of(path).ok_or_else(|| {
                format!(
                    "{}: no channel suffix (expected e.g. name_L.wav or name.L.wav)",
                    path.display()
                )
            })?;
            let index = channel_index(&suffix)
                .ok_or_else(|| format!("{}: unknown channel suffix '{suffix}'", path.display()))?;
            Ok((index, path.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort_by_key(|(index, _)| *index);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavReader, WavWriter};

    const SR: u32 = 48000;

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

    #[test]
    fn routes_five_one_in_dcp_order() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        // distinct constant per channel so we can check the lane placement.
        let fs = 1i32 << 23;
        write_mono(&d.join("mix_L.wav"), fs / 10, 100);
        write_mono(&d.join("mix_R.wav"), fs / 5, 100);
        write_mono(&d.join("mix_C.wav"), fs / 4, 100);
        write_mono(&d.join("mix_Lfe.wav"), fs / 3, 100);
        write_mono(&d.join("mix_Ls.wav"), fs / 2, 100);
        write_mono(&d.join("mix_Rs.wav"), (fs / 3) * 2, 100);

        let out = d.join("routed.wav");
        route_directory(d, &out).unwrap();
        let mut r = WavReader::open(&out).unwrap();
        assert_eq!(r.spec().channels, 6);
        let samples: Vec<i32> = r.samples::<i32>().map(|x| x.unwrap()).collect();
        // first frame: L,R,C,LFE,Ls,Rs in order.
        let f0 = &samples[..6];
        assert_eq!(f0[0], fs / 10);
        assert_eq!(f0[1], fs / 5);
        assert_eq!(f0[2], fs / 4);
        assert_eq!(f0[3], fs / 3);
        assert_eq!(f0[4], fs / 2);
        assert_eq!(f0[5], (fs / 3) * 2);
    }

    #[test]
    fn missing_lfe_leaves_silent_lane() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let fs = 1i32 << 23;
        write_mono(&d.join("m_L.wav"), fs / 10, 50);
        write_mono(&d.join("m_R.wav"), fs / 5, 50);
        write_mono(&d.join("m_Ls.wav"), fs / 2, 50); // index 4 -> 5 channels
        write_mono(&d.join("m_Rs.wav"), fs / 3, 50);
        let out = d.join("o.wav");
        route_directory(d, &out).unwrap();
        let mut r = WavReader::open(&out).unwrap();
        assert_eq!(r.spec().channels, 6); // up to Rs (index 5)
        let s: Vec<i32> = r.samples::<i32>().map(|x| x.unwrap()).collect();
        assert_eq!(s[2], 0, "C silent");
        assert_eq!(s[3], 0, "LFE silent");
        assert_eq!(s[4], fs / 2, "Ls present");
    }

    #[test]
    fn dotted_suffix_routes() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let fs = 1i32 << 23;
        write_mono(&d.join("260810 DST_MIX_V2.3.2.L.wav"), fs / 10, 20);
        write_mono(&d.join("260810 DST_MIX_V2.3.2.LFE.wav"), fs / 3, 20);
        assert_eq!(
            channel_stem(&d.join("260810 DST_MIX_V2.3.2.LFE.wav")),
            Some(("260810 DST_MIX_V2.3.2".to_string(), "LFE".to_string()))
        );
        let out = d.join("routed.wav");
        route_directory(d, &out).unwrap();
        let mut r = WavReader::open(&out).unwrap();
        assert_eq!(r.spec().channels, 4);
        let s: Vec<i32> = r.samples::<i32>().map(|x| x.unwrap()).collect();
        assert_eq!(&s[..4], &[fs / 10, 0, 0, fs / 3]);
    }

    #[test]
    fn file_list_routes_like_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let fs = 1i32 << 23;
        write_mono(&d.join("mix-R.wav"), fs / 5, 30);
        write_mono(&d.join("mix L.wav"), fs / 10, 30);
        write_mono(&d.join("mix_C.wav"), fs / 4, 30);
        let from_directory = d.join("from_directory.wav");
        route_directory(d, &from_directory).unwrap();
        let listed = channel_lanes(&[
            d.join("mix_C.wav"),
            d.join("mix L.wav"),
            d.join("mix-R.wav"),
        ])
        .unwrap();
        assert_eq!(routed_channel_count_of(&listed), 3);
        let from_list = d.join("from_list.wav");
        route_files(&listed, &from_list).unwrap();
        assert_eq!(
            std::fs::read(&from_directory).unwrap(),
            std::fs::read(&from_list).unwrap()
        );
    }

    #[test]
    fn rejects_unknown_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        write_mono(&d.join("m_L.wav"), 1, 10);
        write_mono(&d.join("m_Foo.wav"), 1, 10);
        assert!(route_directory(d, &d.join("o.wav")).is_err());
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
        assert!(route_directory(d, &d.join("o.wav")).is_err());
    }
}
