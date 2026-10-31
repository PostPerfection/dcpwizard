//! The picture/sound delay applied to the programme audio before it is
//! wrapped, and the fade filter for the picture.
//!
//! Every adjustment here keeps the running time the same, so the sound still
//! matches the picture duration the CPL declares. Anything that would change
//! the length belongs with the head/tail padding instead, which moves picture
//! and sound together.

use postkit::audio_adjust::check_fade;
use std::path::{Path, PathBuf};

/// Shift the sound against the picture by `delay_ms`, keeping the running time.
/// A positive delay prepends that much silence and drops the same from the tail
/// (the sound arrives later); a negative delay drops from the head and appends
/// silence. A delay at least as long as the programme is refused, since it would
/// leave nothing but silence.
pub fn apply_delay(input: &Path, output: &Path, delay_ms: i64) -> Result<PathBuf, String> {
    if delay_ms == 0 {
        return Ok(input.to_path_buf());
    }
    let info = crate::reel::parse_wav(input)?;
    if info.sample_rate == 0 || info.block_align == 0 {
        return Err(format!("{} has no usable WAV format", input.display()));
    }
    let total_samples = info.data_size / info.block_align as u64;
    let shift = delay_ms.unsigned_abs() * info.sample_rate as u64 / 1000;
    if shift >= total_samples {
        let programme_ms = total_samples * 1000 / info.sample_rate as u64;
        return Err(format!(
            "audio delay of {delay_ms}ms is at least as long as the {programme_ms}ms programme, \
             which would leave silence"
        ));
    }
    let (head_silence, start_sample) = if delay_ms > 0 { (shift, 0) } else { (0, shift) };
    crate::reel::write_shifted_wav(
        input,
        &info,
        head_silence,
        start_sample,
        total_samples,
        output,
    )?;
    Ok(output.to_path_buf())
}

// where a picture fade filter runs against the trim, which decides the clock it reads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeFilterPosition {
    // on the source's clock, ahead of the cut
    BeforeTrim,
    // on the trimmed picture, whose clock starts at the first kept frame
    AfterTrim,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimmedPicture {
    pub source_frames: u64,
    pub trim_start_frames: u64,
    pub trim_end_frames: u64,
    pub fps: f64,
}

/// ffmpeg `-vf` fade chain for the picture, or `None` when neither fade is
/// asked for. The fades start at the first kept frame and end at the last, and
/// are checked against the kept length. Fades darken frames in place, so the
/// frame count the CPL declares is unchanged.
pub fn video_fade_filter(
    fade_in_seconds: Option<f64>,
    fade_out_seconds: Option<f64>,
    picture: &TrimmedPicture,
    position: FadeFilterPosition,
) -> Result<Option<String>, String> {
    if fade_in_seconds.is_none() && fade_out_seconds.is_none() {
        return Ok(None);
    }
    let kept_frames = match picture.trim_start_frames + picture.trim_end_frames {
        0 => picture.source_frames,
        _ => crate::trim::kept_frames(
            picture.source_frames,
            picture.trim_start_frames,
            picture.trim_end_frames,
        )?,
    };
    let kept_seconds = kept_frames as f64 / picture.fps;
    let first_kept_second = match position {
        FadeFilterPosition::BeforeTrim => picture.trim_start_frames as f64 / picture.fps,
        FadeFilterPosition::AfterTrim => 0.0,
    };
    let mut filters = Vec::new();
    if let Some(seconds) = fade_in_seconds {
        check_fade("video fade-in", seconds, kept_seconds)?;
        filters.push(format!("fade=t=in:st={first_kept_second}:d={seconds}"));
    }
    if let Some(seconds) = fade_out_seconds {
        check_fade("video fade-out", seconds, kept_seconds)?;
        let start = first_kept_second + kept_seconds - seconds;
        filters.push(format!("fade=t=out:st={start}:d={seconds}"));
    }
    Ok(Some(filters.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DURATION: f64 = 10.0;

    const UNTRIMMED: TrimmedPicture = TrimmedPicture {
        source_frames: 240,
        trim_start_frames: 0,
        trim_end_frames: 0,
        fps: 24.0,
    };

    #[test]
    fn video_fades_build_a_chain_and_none_when_unasked() {
        for position in [
            FadeFilterPosition::BeforeTrim,
            FadeFilterPosition::AfterTrim,
        ] {
            assert_eq!(
                video_fade_filter(None, None, &UNTRIMMED, position).unwrap(),
                None
            );
            assert_eq!(
                video_fade_filter(Some(1.0), Some(2.0), &UNTRIMMED, position).unwrap(),
                Some("fade=t=in:st=0:d=1,fade=t=out:st=8:d=2".to_string())
            );
        }
        let err = video_fade_filter(
            Some(DURATION + 1.0),
            None,
            &UNTRIMMED,
            FadeFilterPosition::AfterTrim,
        )
        .unwrap_err();
        assert!(err.contains("longer than"), "got: {err}");
    }

    // 10 s of source with 2 s trimmed off the head and 3 s off the tail keeps 5 s
    #[test]
    fn video_fades_start_and_end_on_the_kept_frames() {
        let trimmed = TrimmedPicture {
            trim_start_frames: 48,
            trim_end_frames: 72,
            ..UNTRIMMED
        };
        assert_eq!(
            video_fade_filter(
                Some(1.0),
                Some(1.0),
                &trimmed,
                FadeFilterPosition::BeforeTrim
            )
            .unwrap()
            .as_deref(),
            Some("fade=t=in:st=2:d=1,fade=t=out:st=6:d=1")
        );
        assert_eq!(
            video_fade_filter(
                Some(1.0),
                Some(1.0),
                &trimmed,
                FadeFilterPosition::AfterTrim
            )
            .unwrap()
            .as_deref(),
            Some("fade=t=in:st=0:d=1,fade=t=out:st=4:d=1")
        );
        let err = video_fade_filter(Some(6.0), None, &trimmed, FadeFilterPosition::BeforeTrim)
            .unwrap_err();
        assert!(err.contains("longer than the 5s"), "got: {err}");
        assert!(
            video_fade_filter(
                Some(1.0),
                None,
                &TrimmedPicture {
                    trim_end_frames: 192,
                    ..trimmed
                },
                FadeFilterPosition::AfterTrim
            )
            .is_err(),
            "a trim that keeps nothing has no frames to fade"
        );
    }

    /// A 48 kHz mono ramp of `samples`, and the sample count of a WAV.
    fn ramp(dir: &std::path::Path, name: &str, samples: u64) -> PathBuf {
        let path = dir.join(name);
        crate::test_wav::write_ramp_wav(&path, 1, samples);
        path
    }

    fn sample_count(path: &Path) -> u64 {
        let info = crate::reel::parse_wav(path).unwrap();
        info.data_size / info.block_align as u64
    }

    fn sample_at(path: &Path, index: u64) -> u16 {
        let info = crate::reel::parse_wav(path).unwrap();
        let bytes = std::fs::read(path).unwrap();
        let at = (info.data_offset + index * info.block_align as u64) as usize;
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    }

    // the running-time invariant this whole module is built on: a delay may not
    // change how many samples ship, only which ones
    #[test]
    fn a_delay_keeps_the_sample_count() {
        let dir = tempfile::tempdir().unwrap();
        let src = ramp(dir.path(), "in.wav", 48_000);
        for delay_ms in [250i64, -250, 1, -1] {
            let out = dir.path().join(format!("out{delay_ms}.wav"));
            apply_delay(&src, &out, delay_ms).unwrap();
            assert_eq!(
                sample_count(&out),
                48_000,
                "a {delay_ms}ms delay must not change the running time"
            );
        }
    }

    #[test]
    fn a_positive_delay_leads_with_silence_and_a_negative_one_starts_late() {
        let dir = tempfile::tempdir().unwrap();
        let src = ramp(dir.path(), "in.wav", 48_000);

        // +250ms is 12000 samples of silence, then the ramp from its own zero
        let later = dir.path().join("later.wav");
        apply_delay(&src, &later, 250).unwrap();
        assert_eq!(sample_at(&later, 11_999), 0, "lead-in is silent");
        assert_eq!(sample_at(&later, 12_000), 0, "the ramp restarts at zero");
        assert_eq!(
            sample_at(&later, 12_100),
            100,
            "the ramp follows the silence"
        );

        // -250ms drops the first 12000 samples and appends the same in silence
        let earlier = dir.path().join("earlier.wav");
        apply_delay(&src, &earlier, -250).unwrap();
        assert_eq!(sample_at(&earlier, 0), 12_000);
        assert_eq!(
            sample_at(&earlier, 47_999),
            0,
            "the tail is made up in silence"
        );
    }

    #[test]
    fn a_delay_longer_than_the_programme_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let src = ramp(dir.path(), "in.wav", 48_000);
        let err = apply_delay(&src, &dir.path().join("out.wav"), 2_000).unwrap_err();
        assert!(err.contains("leave silence"), "got: {err}");
        assert!(apply_delay(&src, &dir.path().join("out.wav"), -2_000).is_err());
    }

    #[test]
    fn a_zero_delay_leaves_the_input_alone() {
        let input = Path::new("in.wav");
        assert_eq!(
            apply_delay(input, Path::new("out.wav"), 0).unwrap(),
            input.to_path_buf(),
            "nothing to do must not rewrite the audio"
        );
    }

    // a gain of zero is a real request (it is what --audio-gain 0 means), so it
}
