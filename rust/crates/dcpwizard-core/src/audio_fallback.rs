//! The sound a build uses when the job named no audio file: the picture
//! source's own track, or digital silence as long as the picture.
//!
//! Both produce the 48 kHz 24-bit PCM a DCP sound track carries, so whatever
//! comes out of here can be processed and wrapped like a supplied WAV.

use std::path::{Path, PathBuf};

const SAMPLE_RATE: u32 = 48_000;
const BITS_PER_SAMPLE: u16 = 24;

/// Pull the video's own audio out to a 48 kHz 24-bit WAV in `work_dir`, every
/// channel as the source carries it. None when the source has no audio stream.
pub fn extract_embedded_audio(video: &Path, work_dir: &Path) -> Result<Option<PathBuf>, String> {
    if !crate::probe::probe_video(video).is_some_and(|info| info.has_audio) {
        return Ok(None);
    }
    std::fs::create_dir_all(work_dir)
        .map_err(|e| format!("cannot create {}: {e}", work_dir.display()))?;
    let output = work_dir.join("embedded.wav");
    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(video)
        .arg("-vn")
        .args(["-c:a", "pcm_s24le", "-ar", &SAMPLE_RATE.to_string()])
        .args(["-rf64", "auto"])
        .arg(&output)
        .output()
        .map_err(|e| format!("failed to run ffmpeg to extract the source's audio: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "could not extract the audio from {}: {}",
            video.display(),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(Some(output))
}

/// Write `frames` frames' worth of 48 kHz 24-bit silence across `channels`,
/// sample-accurate at the frame edge so it lines up with the picture.
pub fn write_silent_wav(output: &Path, channels: u32, frames: u64, fps: u32) -> Result<(), String> {
    crate::pad::check_frame_aligned_sample_rate(SAMPLE_RATE, fps)?;
    let spec = postkit::wav_io::WavSpec {
        channels: channels as u16,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: BITS_PER_SAMPLE,
        sample_format: postkit::wav_io::SampleFormat::Int,
    };
    let sample_frames = frames * (SAMPLE_RATE / fps) as u64;
    let cannot_write = |e: std::io::Error| format!("cannot write {output:?}: {e}");
    let mut writer = postkit::wav_io::WavWriter::create_plain_pcm(output, spec, sample_frames)
        .map_err(cannot_write)?;
    let zeros = vec![0u8; 1 << 16];
    let mut remaining = sample_frames * (BITS_PER_SAMPLE / 8) as u64 * channels as u64;
    while remaining > 0 {
        let take = remaining.min(zeros.len() as u64) as usize;
        writer.write_bytes(&zeros[..take]).map_err(cannot_write)?;
        remaining -= take as u64;
    }
    writer.finalize().map_err(cannot_write)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_as_long_as_the_picture() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("silence.wav");
        write_silent_wav(&wav, 6, 48, 24).expect("write the silence");

        let reader = hound::WavReader::open(&wav).unwrap();
        assert_eq!(reader.spec().channels, 6);
        assert_eq!(reader.spec().sample_rate, SAMPLE_RATE);
        assert_eq!(reader.spec().bits_per_sample, BITS_PER_SAMPLE);
        assert_eq!(reader.duration(), 48 * 2000, "two seconds at 24 fps");
        assert!(
            reader.into_samples::<i32>().all(|s| s.unwrap() == 0),
            "every sample must be silent"
        );
    }

    #[test]
    fn a_rate_that_does_not_divide_by_the_frame_rate_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        assert!(write_silent_wav(&dir.path().join("silence.wav"), 6, 48, 7).is_err());
    }
}
