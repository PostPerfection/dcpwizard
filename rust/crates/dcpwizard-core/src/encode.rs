use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Encode configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EncodeConfig {
    pub bandwidth_mbps: u32,
    pub threads: u32,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

/// Encode image sequence to JPEG 2000 codestream using grk_compress.
pub fn encode_j2k(config: &EncodeConfig) -> i32 {
    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    // Find input frames
    let mut frames: Vec<PathBuf> = std::fs::read_dir(&config.input_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "tif" | "tiff" | "dpx" | "exr" | "png" | "bmp"))
        })
        .collect();
    frames.sort();

    if frames.is_empty() {
        tracing::error!("No image frames found in {}", config.input_dir.display());
        return -1;
    }

    let bandwidth = if config.bandwidth_mbps > 0 {
        config.bandwidth_mbps
    } else {
        250
    };

    for frame in &frames {
        let stem = frame
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("frame");
        let output = config.output_dir.join(format!("{stem}.j2c"));

        let result = std::process::Command::new("grk_compress")
            .arg("-i")
            .arg(frame)
            .arg("-o")
            .arg(&output)
            .arg("-r")
            .arg(format!("{}", bandwidth))
            .arg("-cinema2K")
            .arg("24")
            .output();

        match result {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                tracing::error!(
                    "grk_compress failed for {}: {}",
                    stem,
                    String::from_utf8_lossy(&o.stderr)
                );
                return -1;
            }
            Err(e) => {
                tracing::error!("Failed to run grk_compress: {e}");
                return -1;
            }
        }
    }

    tracing::info!(
        "Encoded {} frames to J2K at {} Mbps",
        frames.len(),
        bandwidth
    );
    0
}
