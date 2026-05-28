use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Export format for transcoding DCP content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExportFormat {
    ProRes,
    #[default]
    H264,
    H265,
    DnxHr,
    ImageSequence,
}

impl ExportFormat {
    fn ffmpeg_codec(&self) -> &'static str {
        match self {
            ExportFormat::ProRes => "prores_ks",
            ExportFormat::H264 => "libx264",
            ExportFormat::H265 => "libx265",
            ExportFormat::DnxHr => "dnxhd",
            ExportFormat::ImageSequence => "png",
        }
    }

    fn file_extension(&self) -> &'static str {
        match self {
            ExportFormat::ProRes => "mov",
            ExportFormat::H264 => "mp4",
            ExportFormat::H265 => "mp4",
            ExportFormat::DnxHr => "mxf",
            ExportFormat::ImageSequence => "png",
        }
    }
}

/// Export configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportConfig {
    pub input_mxf: PathBuf,
    pub output_path: PathBuf,
    pub format: ExportFormat,
    pub quality_crf: u32,
    pub audio_mxf: Option<PathBuf>,
}

/// Export / transcode DCP MXF content to a delivery format via ffmpeg.
pub fn export_dcp(config: &ExportConfig) -> i32 {
    if !config.input_mxf.exists() {
        tracing::error!("Input MXF not found: {}", config.input_mxf.display());
        return -1;
    }

    let crf = if config.quality_crf == 0 {
        18
    } else {
        config.quality_crf
    };

    if config.format == ExportFormat::ImageSequence {
        return export_image_sequence(&config.input_mxf, &config.output_path);
    }

    let output = if config.output_path.extension().is_none() {
        config
            .output_path
            .with_extension(config.format.file_extension())
    } else {
        config.output_path.clone()
    };

    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(&config.input_mxf);

    if let Some(audio) = &config.audio_mxf
        && audio.exists()
    {
        cmd.arg("-i").arg(audio);
    }

    cmd.arg("-c:v").arg(config.format.ffmpeg_codec());

    match config.format {
        ExportFormat::H264 | ExportFormat::H265 => {
            cmd.arg("-crf").arg(crf.to_string());
            cmd.arg("-preset").arg("medium");
        }
        ExportFormat::ProRes => {
            cmd.arg("-profile:v").arg("3"); // ProRes HQ
        }
        ExportFormat::DnxHr => {
            cmd.arg("-profile:v").arg("dnxhr_hq");
        }
        ExportFormat::ImageSequence => unreachable!(),
    }

    cmd.arg("-c:a").arg("aac").arg(&output);

    let result = cmd.output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Exported DCP to {}", output.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg export failed: {}",
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

/// Extract a single frame from an MXF at the given frame number.
pub fn extract_frame(mxf_path: &Path, frame_number: u64, output_path: &Path) -> i32 {
    let fps = 24; // Default DCP frame rate
    let timestamp = format!(
        "{:02}:{:02}:{:02}.{:03}",
        frame_number / (fps * 3600),
        (frame_number / (fps * 60)) % 60,
        (frame_number / fps) % 60,
        ((frame_number % fps) * 1000) / fps
    );

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg(&timestamp)
        .arg("-i")
        .arg(mxf_path)
        .arg("-frames:v")
        .arg("1")
        .arg(output_path)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!(
                "Extracted frame {} to {}",
                frame_number,
                output_path.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg frame extraction failed: {}",
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

fn export_image_sequence(input_mxf: &Path, output_dir: &Path) -> i32 {
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let pattern = output_dir.join("frame_%08d.png");

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(input_mxf)
        .arg(&pattern)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Exported image sequence to {}", output_dir.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg image sequence export failed: {}",
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
