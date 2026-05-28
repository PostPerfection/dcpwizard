use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use postkit::grok;
use postkit::grok_encoder::{self, CompressParams, RawFrame};

/// Encode configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EncodeConfig {
    pub bandwidth_mbps: u32,
    pub threads: u32,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

/// Encode image sequence to JPEG 2000 using in-process Grok FFI pipeline.
pub fn encode_j2k(config: &EncodeConfig) -> i32 {
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

    let ratio = if config.bandwidth_mbps > 0 {
        // Convert target Mbps to compression ratio
        // DCI 2K 24fps: uncompressed ≈ 2048*1080*3*12*24 bits/sec ≈ 2.28 Gbps
        // ratio = uncompressed_bps / target_bps
        let uncompressed_mbps = 2048.0 * 1080.0 * 3.0 * 12.0 * 24.0 / 1_000_000.0;
        uncompressed_mbps / config.bandwidth_mbps as f64
    } else {
        10.0
    };

    let params = CompressParams {
        compression_ratio: ratio,
        ..CompressParams::default()
    };

    let total_frames = frames.len() as u64;
    let cancel = Arc::new(AtomicBool::new(false));
    let mut frame_iter = frames.into_iter().enumerate();

    grok_encoder::initialize(0);

    let result = grok_encoder::encode_pipeline(
        &config.output_dir,
        &params,
        total_frames,
        &cancel,
        || {
            let (idx, path) = frame_iter.next()?;
            match grok::load_tiff(&path) {
                Ok(tf) => Some(RawFrame::Planar {
                    components: tf.components,
                    width: tf.width,
                    height: tf.height,
                    precision: tf.precision,
                    index: idx as u64,
                }),
                Err(e) => {
                    tracing::error!("Failed to load {}: {e}", path.display());
                    None
                }
            }
        },
        |progress| {
            if progress.total_frames > 0 {
                tracing::info!(
                    "Encoding: {}/{} frames ({:.1} fps)",
                    progress.frames_encoded,
                    progress.total_frames,
                    progress.fps,
                );
            }
        },
    );

    grok_encoder::deinitialize();

    if !result.success {
        tracing::error!("Encode failed: {}", result.error);
        return -1;
    }

    tracing::info!(
        "Encoded {} frames to J2K (ratio {:.1}:1)",
        result.frames_encoded,
        ratio,
    );
    0
}
