use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Scaling/framing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScaleMode {
    #[default]
    None,
    Scale,
    Crop,
    Letterbox,
    PillarBox,
}

/// Geometry / framing configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeometryConfig {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
    pub mode: ScaleMode,
    pub target_width: u32,
    pub target_height: u32,
}

/// Apply geometry transform using ffmpeg scale/crop/pad filters.
pub fn apply_geometry(config: &GeometryConfig) -> i32 {
    if config.target_width == 0 || config.target_height == 0 {
        tracing::error!("Target dimensions must be non-zero");
        return -1;
    }

    let filter = build_filter(config);

    if filter.is_empty() {
        // No transform needed, just copy
        match std::fs::copy(&config.input_file, &config.output_file) {
            Ok(_) => return 0,
            Err(e) => {
                tracing::error!("Failed to copy file: {e}");
                return -1;
            }
        }
    }

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&config.input_file)
        .arg("-vf")
        .arg(&filter)
        .arg("-c:a")
        .arg("copy")
        .arg(&config.output_file)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!(
                "Applied {:?} geometry to {}",
                config.mode,
                config.output_file.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg geometry transform failed: {}",
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

fn build_filter(config: &GeometryConfig) -> String {
    let w = config.target_width;
    let h = config.target_height;

    match config.mode {
        ScaleMode::None => String::new(),
        ScaleMode::Scale => {
            format!("scale={w}:{h}:flags=lanczos")
        }
        ScaleMode::Crop => {
            // Scale to cover target, then crop to exact size
            format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}")
        }
        ScaleMode::Letterbox => {
            // Scale to fit within target, then pad with black bars top/bottom
            format!(
                "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
            )
        }
        ScaleMode::PillarBox => {
            // Scale to fit height, then pad with black bars left/right
            format!(
                "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black"
            )
        }
    }
}
