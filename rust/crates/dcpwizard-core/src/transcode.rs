use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Transcode configuration for video to image sequence conversion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscodeConfig {
    pub input_file: PathBuf,
    pub output_dir: PathBuf,
    pub image_format: String,
    pub target_width: u32,
    pub target_height: u32,
    pub target_fps: u32,
    pub pixel_format: String,
}

/// Transcode video to an image sequence using ffmpeg.
pub fn transcode_to_sequence(config: &TranscodeConfig) -> i32 {
    if !config.input_file.exists() {
        tracing::error!("Input file not found: {}", config.input_file.display());
        return -1;
    }

    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let ext = match config.image_format.as_str() {
        "dpx" => "dpx",
        "exr" => "exr",
        "png" => "png",
        "bmp" => "bmp",
        "tiff" | "tif" | "" => "tiff",
        other => {
            tracing::warn!("Unknown image format '{other}', defaulting to tiff");
            "tiff"
        }
    };

    let fps = if config.target_fps == 0 {
        24
    } else {
        config.target_fps
    };

    let pattern = config.output_dir.join(format!("frame_%08d.{ext}"));

    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(&config.input_file);

    // Build filter chain
    let mut filters = Vec::new();
    if config.target_width > 0 && config.target_height > 0 {
        filters.push(format!(
            "scale={}:{}:flags=lanczos",
            config.target_width, config.target_height
        ));
    }
    filters.push(format!("fps={fps}"));

    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
    }

    if !config.pixel_format.is_empty() {
        cmd.arg("-pix_fmt").arg(&config.pixel_format);
    } else if ext == "tiff" || ext == "dpx" {
        cmd.arg("-pix_fmt").arg("rgb48le");
    }

    // Disable audio
    cmd.arg("-an");
    cmd.arg(&pattern);

    let result = cmd.output();

    match result {
        Ok(o) if o.status.success() => {
            let frame_count = std::fs::read_dir(&config.output_dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e == ext)
                })
                .count();
            tracing::info!(
                "Transcoded {frame_count} frames to {}",
                config.output_dir.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg transcode failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            -1
        }
    }
}
