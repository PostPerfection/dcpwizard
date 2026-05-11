use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// DTS:X wrapping configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DtsxConfig {
    pub input_file: PathBuf,
    pub output_mxf: PathBuf,
    pub fps: u32,
}

/// Wrap DTS:X essence into DCP-compliant MXF using asdcp-wrap.
pub fn wrap_dtsx(config: &DtsxConfig) -> i32 {
    if !config.input_file.exists() {
        tracing::error!(
            "DTS:X input file not found: {}",
            config.input_file.display()
        );
        return -1;
    }

    let fps = if config.fps == 0 { 24 } else { config.fps };

    let result = std::process::Command::new("asdcp-wrap")
        .arg("-p")
        .arg(fps.to_string())
        .arg("-y")
        .arg(&config.input_file)
        .arg(&config.output_mxf)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("Wrapped DTS:X to MXF: {}", config.output_mxf.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "asdcp-wrap DTS:X failed: {}",
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
