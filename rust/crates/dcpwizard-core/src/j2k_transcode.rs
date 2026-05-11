use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// J2K transcoding configuration for re-encoding at different bitrate/parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct J2kTranscodeConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub target_bitrate_mbps: u32,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub cinema_profile: String,
}

/// Re-encode JPEG 2000 codestream files at a different bitrate using grk_compress.
///
/// Input: directory of J2K files. Each frame is decoded and re-encoded.
pub fn transcode_j2k(config: &J2kTranscodeConfig) -> i32 {
    if !config.input_dir.exists() {
        tracing::error!(
            "Input J2K directory not found: {}",
            config.input_dir.display()
        );
        return -1;
    }

    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let mut frames: Vec<PathBuf> = std::fs::read_dir(&config.input_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "j2c" | "j2k"))
        })
        .collect();
    frames.sort();

    if frames.is_empty() {
        tracing::error!("No J2K frames found in {}", config.input_dir.display());
        return -1;
    }

    let bitrate = if config.target_bitrate_mbps == 0 {
        250
    } else {
        config.target_bitrate_mbps
    };

    let profile = if config.cinema_profile.is_empty() {
        "cinema2K"
    } else {
        &config.cinema_profile
    };

    let total = frames.len();
    tracing::info!("Re-encoding {total} J2K frames at {bitrate} Mbps");

    for (i, frame) in frames.iter().enumerate() {
        let stem = frame
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("frame");
        let output = config.output_dir.join(format!("{stem}.j2c"));

        // grk_decompress to intermediate, then grk_compress
        let tmp_tiff = config.output_dir.join(format!("{stem}_tmp.tif"));

        // Decompress J2K → TIFF
        let decompress = std::process::Command::new("grk_decompress")
            .arg("-i")
            .arg(frame)
            .arg("-o")
            .arg(&tmp_tiff)
            .output();

        match &decompress {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                tracing::error!(
                    "grk_decompress failed for {}: {}",
                    frame.display(),
                    String::from_utf8_lossy(&o.stderr)
                );
                let _ = std::fs::remove_file(&tmp_tiff);
                return -1;
            }
            Err(e) => {
                tracing::error!("Failed to run grk_decompress: {e}");
                return -1;
            }
        }

        // Re-compress TIFF → J2K at new bitrate
        let mut compress_cmd = std::process::Command::new("grk_compress");
        compress_cmd
            .arg("-i")
            .arg(&tmp_tiff)
            .arg("-o")
            .arg(&output)
            .arg("-r")
            .arg(bitrate.to_string())
            .arg(format!("-{profile}"))
            .arg("24");

        if config.resolution_width > 0 && config.resolution_height > 0 {
            compress_cmd
                .arg("-W")
                .arg(config.resolution_width.to_string())
                .arg("-H")
                .arg(config.resolution_height.to_string());
        }

        let compress = compress_cmd.output();
        let _ = std::fs::remove_file(&tmp_tiff);

        match compress {
            Ok(o) if o.status.success() => {
                if (i + 1) % 100 == 0 || i + 1 == total {
                    tracing::info!("Re-encoded [{}/{}] frames", i + 1, total);
                }
            }
            Ok(o) => {
                tracing::error!(
                    "grk_compress failed for {}: {}",
                    frame.display(),
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
        "Re-encoded {total} frames to {}",
        config.output_dir.display()
    );
    0
}
