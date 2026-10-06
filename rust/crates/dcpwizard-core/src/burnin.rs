//! Subtitle / text burn-in via ffmpeg drawtext.
//!
//! Delegates to [`postkit::burnin`], which also handles subtitle-file burn-in
//! (SRT/ASS/SMPTE) in addition to plain text overlays.

pub use postkit::burnin::BurninOptions;

use crate::subtitle_extract::PackagedTrack;
use crate::subtitle_preview::{PACKAGED_EXTENSIONS, playable_subtitle_file};
use std::path::Path;

pub fn burnin(opts: &BurninOptions) -> Result<(), String> {
    let Some(subtitle) = opts
        .subtitle_file
        .as_deref()
        .filter(|subtitle| is_packaged_subtitle(subtitle))
    else {
        return postkit::burnin::burnin(opts).map_err(|e| e.to_string());
    };
    let work_dir = tempfile::tempdir()
        .map_err(|e| format!("cannot create a folder for the converted subtitles: {e}"))?;
    let srt = playable_subtitle_file(subtitle, PackagedTrack::Subtitle, work_dir.path())?;
    let converted = BurninOptions {
        subtitle_file: Some(srt),
        ..opts.clone()
    };
    postkit::burnin::burnin(&converted).map_err(|e| e.to_string())
}

fn is_packaged_subtitle(subtitle: &Path) -> bool {
    subtitle
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| PACKAGED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const FRAME_RATE: u32 = 24;
    const CLIP_SIZE: &str = "320x240";
    const CLIP_SECONDS: u32 = 2;
    const LOSSLESS_CODEC: &str = "ffv1";
    // inside the cue, which runs from 0.5 s to 1.5 s
    const SECONDS_INTO_CUE: &str = "1.0";

    const DCST_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<dcst:SubtitleReel xmlns:dcst="http://www.smpte-ra.org/schemas/428-7/2010/DCST">
  <dcst:TimeCodeRate>24</dcst:TimeCodeRate>
  <dcst:SubtitleList>
    <dcst:Font>
      <dcst:Subtitle SpotNumber="1" TimeIn="00:00:00:12" TimeOut="00:00:01:12">
        <dcst:Text>Burnt line</dcst:Text>
      </dcst:Subtitle>
    </dcst:Font>
  </dcst:SubtitleList>
</dcst:SubtitleReel>"#;

    fn black_clip(dir: &Path) -> PathBuf {
        let clip = dir.join("black.mkv");
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
            .arg(format!(
                "color=black:size={CLIP_SIZE}:rate={FRAME_RATE}:duration={CLIP_SECONDS}"
            ))
            .args(["-c:v", LOSSLESS_CODEC])
            .arg(&clip)
            .status()
            .expect("ffmpeg runs");
        assert!(status.success());
        clip
    }

    fn grey_frame_at(video: &Path, seconds: &str) -> Vec<u8> {
        let output = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-ss", seconds, "-i"])
            .arg(video)
            .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray", "-"])
            .output()
            .expect("ffmpeg runs");
        assert!(
            output.status.success(),
            "{} does not decode",
            video.display()
        );
        assert!(
            !output.stdout.is_empty(),
            "no frame decoded from {}",
            video.display()
        );
        output.stdout
    }

    #[test]
    fn a_dcst_xml_subtitle_burns_into_the_frames_it_covers() {
        let dir = tempfile::tempdir().unwrap();
        let clip = black_clip(dir.path());
        let subtitle = dir.path().join("subs.xml");
        std::fs::write(&subtitle, DCST_XML).unwrap();
        let burnt = dir.path().join("burnt.mkv");
        let plain = dir.path().join("plain.mkv");
        let options = |output: &Path, subtitle_file: Option<PathBuf>| BurninOptions {
            input: clip.clone(),
            output: output.to_path_buf(),
            subtitle_file,
            video_codec: LOSSLESS_CODEC.into(),
            ..Default::default()
        };

        burnin(&options(&burnt, Some(subtitle))).unwrap();
        burnin(&options(&plain, None)).unwrap();

        assert_ne!(
            grey_frame_at(&burnt, SECONDS_INTO_CUE),
            grey_frame_at(&plain, SECONDS_INTO_CUE),
            "the cue's text is missing from the burnt frame"
        );
    }
}
