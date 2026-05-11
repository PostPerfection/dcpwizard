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

/// Import video and extract image sequence + audio using ffmpeg.
pub fn import_video(config: &ImportConfig) -> i32 {
    if !config.input_file.exists() {
        tracing::error!("Input file not found: {}", config.input_file.display());
        return -1;
    }

    if let Err(e) = std::fs::create_dir_all(&config.image_output_dir) {
        tracing::error!("Failed to create image output directory: {e}");
        return -1;
    }

    let img_format = if config.image_format.is_empty() {
        "tiff"
    } else {
        &config.image_format
    };
    let ext = match img_format {
        "dpx" => "dpx",
        "exr" => "exr",
        "png" => "png",
        "bmp" => "bmp",
        _ => "tiff",
    };

    let fps = if config.target_fps == 0 {
        24
    } else {
        config.target_fps
    };

    // Extract image sequence
    let pattern = config.image_output_dir.join(format!("frame_%08d.{ext}"));

    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(&config.input_file);

    let mut filters = Vec::new();
    if config.target_width > 0 && config.target_height > 0 {
        filters.push(format!(
            "scale={}:{}:flags=lanczos",
            config.target_width, config.target_height
        ));
    }
    filters.push(format!("fps={fps}"));

    cmd.arg("-vf").arg(filters.join(","));

    if ext == "tiff" {
        cmd.arg("-pix_fmt").arg("rgb48le");
    }

    cmd.arg(&pattern);

    let img_result = cmd.output();
    match &img_result {
        Ok(o) if o.status.success() => {
            tracing::info!(
                "Extracted image sequence to {}",
                config.image_output_dir.display()
            );
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg image extraction failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return -1;
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            return -1;
        }
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
