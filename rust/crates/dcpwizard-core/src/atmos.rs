use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Dolby Atmos IAB wrapping configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtmosConfig {
    pub input_iab: PathBuf,
    pub output_mxf: PathBuf,
    pub fps: u32,
}

/// Wrap Dolby Atmos IAB essence into DCP-compliant MXF using asdcp-wrap.
pub fn wrap_atmos(config: &AtmosConfig) -> i32 {
    if !config.input_iab.exists() {
        tracing::error!("IAB input file not found: {}", config.input_iab.display());
        return -1;
    }

    let fps = if config.fps == 0 { 24 } else { config.fps };

    let result = std::process::Command::new("asdcp-wrap")
        .arg("-p")
        .arg(fps.to_string())
        .arg("-y")
        .arg(&config.input_iab)
        .arg(&config.output_mxf)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Wrapped Atmos IAB to MXF: {}", config.output_mxf.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "asdcp-wrap Atmos failed: {}",
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
