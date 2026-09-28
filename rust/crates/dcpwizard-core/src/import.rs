use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Import configuration for ingesting video into DCP pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportConfig {
    pub input_file: PathBuf,
    pub image_output_dir: PathBuf,
    pub audio_output_file: PathBuf,
    pub image_format: String,
    pub target_width: u32,
    pub target_height: u32,
    pub target_fps: u32,
}

const DEFAULT_IMAGE_FORMAT: &str = "tiff";

/// Import video and extract image sequence + audio using ffmpeg.
pub fn import_video(config: &ImportConfig) -> i32 {
    let image_format = if config.image_format.is_empty() {
        DEFAULT_IMAGE_FORMAT.to_string()
    } else {
        config.image_format.clone()
    };
    let frames = crate::transcode::TranscodeConfig {
        input_file: config.input_file.clone(),
        output_dir: config.image_output_dir.clone(),
        image_format,
        target_width: config.target_width,
        target_height: config.target_height,
        target_fps: config.target_fps,
        pixel_format: String::new(),
    };
    let frames_exit_code = crate::transcode::transcode_to_sequence(&frames);
    if frames_exit_code != 0 {
        return frames_exit_code;
    }

    // Extract audio as 24-bit 48kHz WAV
    if let Some(parent) = config.audio_output_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let audio_result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&config.input_file)
        .arg("-vn")
        .arg("-acodec")
        .arg("pcm_s24le")
        .arg("-ar")
        .arg("48000")
        .arg(&config.audio_output_file)
        .output();

    match audio_result {
        Ok(o) if o.status.success() => {
            tracing::info!("Extracted audio to {}", config.audio_output_file.display());
        }
        Ok(o) => {
            // Audio extraction may fail if source has no audio — treat as warning
            tracing::warn!(
                "ffmpeg audio extraction issue: {}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
        Err(e) => {
            tracing::warn!("Could not extract audio: {e}");
        }
    }

    0
}

/// Return the list of supported input video formats.
pub fn supported_formats() -> Vec<String> {
    vec![
        "mov".into(),
        "mp4".into(),
        "mkv".into(),
        "avi".into(),
        "mxf".into(),
        "prores".into(),
        "dnxhd".into(),
        "dpx".into(),
        "tiff".into(),
        "exr".into(),
        "wav".into(),
        "aiff".into(),
        "flac".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_SECONDS: u32 = 2;
    const SOURCE_FRAMES_PER_SECOND: u32 = 25;

    fn source_clip(directory: &std::path::Path) -> PathBuf {
        let clip = directory.join("source.mp4");
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
            .arg(format!(
                "testsrc2=size=64x36:rate={SOURCE_FRAMES_PER_SECOND}:duration={SOURCE_SECONDS}"
            ))
            .args(["-f", "lavfi", "-i"])
            .arg(format!("sine=frequency=440:duration={SOURCE_SECONDS}"))
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
            .arg(&clip)
            .status()
            .expect("ffmpeg runs");
        assert!(status.success());
        clip
    }

    #[test]
    fn import_keeps_the_source_frame_rate_and_writes_the_audio() {
        let directory = tempfile::tempdir().unwrap();
        let config = ImportConfig {
            input_file: source_clip(directory.path()),
            image_output_dir: directory.path().join("frames"),
            audio_output_file: directory.path().join("audio.wav"),
            image_format: "png".to_string(),
            ..Default::default()
        };

        assert_eq!(import_video(&config), 0);

        let stills = std::fs::read_dir(&config.image_output_dir)
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    == Some("png")
            })
            .count();
        assert_eq!(stills, (SOURCE_SECONDS * SOURCE_FRAMES_PER_SECOND) as usize);
        assert!(config.audio_output_file.is_file());
    }

    #[test]
    fn import_refuses_an_image_format_it_does_not_know() {
        let directory = tempfile::tempdir().unwrap();
        let config = ImportConfig {
            input_file: source_clip(directory.path()),
            image_output_dir: directory.path().join("frames"),
            audio_output_file: directory.path().join("audio.wav"),
            image_format: "jpeg2000".to_string(),
            ..Default::default()
        };

        assert_ne!(import_video(&config), 0);
        assert!(!config.image_output_dir.exists());
    }
}
