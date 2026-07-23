// Filename-based channel auto-routing (dom#2134): given a directory of mono
// WAVs named with a channel suffix (Foo_L.wav, Foo_Lfe.wav, ...), combine them
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

// the channel suffix is the token after the last '_' in the file stem.
fn suffix_of(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.rsplit_once('_'))
        .map(|(_, suf)| suf.to_string())
}

/// Route every `*.wav` in `dir` to its channel lane and write one interleaved
/// multichannel WAV to `output`. Every file must be mono, share sample rate /
/// bit depth / format, and carry a recognized channel suffix; anything else
/// fails loud. The output channel count is the highest routed lane + 1, with
/// unused lanes silent. Returns `output`.
pub fn route_directory(dir: &Path, output: &Path) -> Result<PathBuf, String> {
    let mut entries: Vec<(usize, PathBuf)> = Vec::new();
    let rd = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for e in rd.filter_map(|e| e.ok()) {
        let path = e.path();
        if !path.is_file()
            || path
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.to_lowercase())
                != Some("wav".to_string())
        {
            continue;
        }
        let suffix = suffix_of(&path).ok_or_else(|| {
            format!(
                "{}: no channel suffix (expected e.g. name_L.wav)",
                path.display()
            )
        })?;
        let idx = channel_index(&suffix)
            .ok_or_else(|| format!("{}: unknown channel suffix '{suffix}'", path.display()))?;
        entries.push((idx, path));
    }
    if entries.is_empty() {
        return Err(format!("no channel WAVs found in {}", dir.display()));
    }
    entries.sort_by_key(|(i, _)| *i);

    // read every channel, enforcing mono and a shared format.
    let mut spec: Option<WavSpec> = None;
    let mut lanes: Vec<(usize, Vec<f32>)> = Vec::new();
    for (idx, path) in &entries {
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
                    return Err(format!("channel index {idx} routed twice"));
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
