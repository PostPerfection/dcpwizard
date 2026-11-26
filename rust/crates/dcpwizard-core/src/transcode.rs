use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

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
pub fn transcode_to_sequence(
    config: &TranscodeConfig,
    on_progress: impl FnMut(postkit::transcode::FfmpegProgress),
) -> i32 {
    if !config.input_file.exists() {
        tracing::error!("Input file not found: {}", config.input_file.display());
        return -1;
    }

    let ext = match config.image_format.as_str() {
        "dpx" => "dpx",
        "exr" => "exr",
        "png" => "png",
        "bmp" => "bmp",
        "tiff" | "tif" => "tiff",
        other => {
            tracing::error!("Unknown image format '{other}', use tiff, dpx, exr, png or bmp");
            return -1;
        }
    };

    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let pattern = config.output_dir.join(format!("frame_%08d.{ext}"));

    let mut args: Vec<std::ffi::OsString> =
        vec!["-y".into(), "-i".into(), config.input_file.clone().into()];

    // Build filter chain
    let mut filters = Vec::new();
    if config.target_width > 0 && config.target_height > 0 {
        filters.push(format!(
            "scale={}:{}:flags=lanczos",
            config.target_width, config.target_height
        ));
    }
    if config.target_fps > 0 {
        filters.push(format!("fps={}", config.target_fps));
    }

    if !filters.is_empty() {
        args.extend(["-vf".into(), filters.join(",").into()]);
    }

    if !config.pixel_format.is_empty() {
        args.extend(["-pix_fmt".into(), config.pixel_format.clone().into()]);
    } else if ext == "tiff" || ext == "dpx" {
        args.extend(["-pix_fmt".into(), "rgb48le".into()]);
    }

    // Disable audio
    args.push("-an".into());
    args.push(pattern.clone().into());

    let never_cancelled = AtomicBool::new(false);
    match postkit::transcode::run_ffmpeg_until_cancelled(
        args,
        &config.input_file,
        &pattern,
        &never_cancelled,
        on_progress,
    ) {
        Ok(()) => {
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
        Err(e) => {
            tracing::error!("ffmpeg transcode failed: {e}");
            -1
        }
    }
}
