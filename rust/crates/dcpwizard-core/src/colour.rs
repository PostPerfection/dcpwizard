use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Colour space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColourSpace {
    Rec709,
    #[default]
    P3D65,
    P3Dci,
    Xyz,
}

impl ColourSpace {
    fn ffmpeg_primaries(&self) -> &'static str {
        match self {
            ColourSpace::Rec709 => "bt709",
            ColourSpace::P3D65 => "smpte432",
            ColourSpace::P3Dci => "smpte431",
            ColourSpace::Xyz => "film",
        }
    }

    fn ffmpeg_matrix(&self) -> &'static str {
        match self {
            ColourSpace::Rec709 => "bt709",
            ColourSpace::P3D65 | ColourSpace::P3Dci => "bt709",
            ColourSpace::Xyz => "bt709",
        }
    }
}

/// Transfer function / EOTF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransferFunction {
    #[default]
    Gamma26,
    Pq,
    Hlg,
    Linear,
}

impl TransferFunction {
    fn ffmpeg_trc(&self) -> &'static str {
        match self {
            TransferFunction::Gamma26 => "gamma26",
            TransferFunction::Pq => "smpte2084",
            TransferFunction::Hlg => "arib-std-b67",
            TransferFunction::Linear => "linear",
        }
    }
}

/// Colour conversion configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColourConfig {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
    pub source_space: ColourSpace,
    pub target_space: ColourSpace,
    pub source_transfer: TransferFunction,
    pub target_transfer: TransferFunction,
}

/// Convert colour space and transfer function using ffmpeg's colorspace filter.
pub fn convert_colour(config: &ColourConfig) -> i32 {
    let filter = format!(
        "colorspace=all={}:iall={}:trc={}:itrc={}:primaries={}:iprimaries={}",
        config.target_space.ffmpeg_matrix(),
        config.source_space.ffmpeg_matrix(),
        config.target_transfer.ffmpeg_trc(),
        config.source_transfer.ffmpeg_trc(),
        config.target_space.ffmpeg_primaries(),
        config.source_space.ffmpeg_primaries(),
    );

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
                "Converted colour {:?} → {:?}: {}",
                config.source_space,
                config.target_space,
                config.output_file.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg colour conversion failed: {}",
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
