use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// MXF essence type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MxfType {
    #[default]
    J2kPicture,
    PcmAudio,
    TimedText,
    Atmos,
    DtsX,
}

/// MXF wrapping configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MxfWrapConfig {
    pub input_path: PathBuf,
    pub output_mxf: PathBuf,
    pub mxf_type: MxfType,
    pub frame_rate: u32,
}

/// Wrap essence into an MXF container using asdcp-wrap.
///
/// NOTE: For production use, direct asdcplib FFI binding would be preferred
/// over shelling out. This implementation uses the CLI tool.
pub fn wrap_mxf(config: &MxfWrapConfig) -> i32 {
    if !config.input_path.exists() {
        tracing::error!("Input path not found: {}", config.input_path.display());
        return -1;
    }

    let fps = if config.frame_rate == 0 {
        24
    } else {
        config.frame_rate
    };

    let mut cmd = std::process::Command::new("asdcp-wrap");

    match config.mxf_type {
        MxfType::J2kPicture => {
            cmd.arg("-p").arg(fps.to_string());
        }
        MxfType::PcmAudio => {
            cmd.arg("-a");
        }
        MxfType::TimedText => {
            cmd.arg("-t");
        }
        MxfType::Atmos => {
            cmd.arg("-y").arg("-p").arg(fps.to_string());
        }
        MxfType::DtsX => {
            cmd.arg("-y").arg("-p").arg(fps.to_string());
        }
    }

    cmd.arg(&config.input_path).arg(&config.output_mxf);

    let result = cmd.output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!(
                "Wrapped {:?} to MXF: {}",
                config.mxf_type,
                config.output_mxf.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "asdcp-wrap failed for {:?}: {}",
                config.mxf_type,
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run asdcp-wrap: {e}");
            -1
        }
    }
}
