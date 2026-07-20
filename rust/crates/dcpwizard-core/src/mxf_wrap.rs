//! AS-DCP MXF wrapping.
//!
//! Delegates to [`postkit::mxf_wrap`] (asdcplib FFI). This layer keeps
//! dcpwizard's exit-code API and maps [`MxfType`]/[`MxfWrapConfig`] onto
//! postkit's `EssenceType`/`MxfWrapOptions`. Unlike the previous local
//! implementation, postkit reads the real J2K codestream dimensions (rather
//! than hardcoding 2048x1080) and derives timed-text duration from the
//! subtitle timing.

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
    /// AES-128 content encryption for J2K picture / PCM sound. Not serialized.
    #[serde(skip)]
    pub encryption: Option<postkit::mxf_wrap::MxfEncryption>,
}

/// Collect sorted files from a directory, or treat a single file as one-element list.
fn collect_inputs(path: &std::path::Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if path.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| format!("cannot read dir {}: {e}", path.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(format!("no files in {}", path.display()));
        }
        return Ok(files);
    }
    Err(format!("input path not found: {}", path.display()))
}

/// Wrap essence into an MXF and return the track file (real embedded asset id,
/// hash, size, duration). `None` on input-collection or wrap failure.
pub fn wrap_mxf_result(config: &MxfWrapConfig) -> Option<postkit::mxf_wrap::MxfTrackFile> {
    let input_files = match collect_inputs(&config.input_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("{e}");
            return None;
        }
    };

    // DTS:X shares the Atmos (IAB data essence) wrapper path.
    let essence_type = match config.mxf_type {
        MxfType::J2kPicture => postkit::mxf_wrap::EssenceType::J2k,
        MxfType::PcmAudio => postkit::mxf_wrap::EssenceType::Pcm,
        MxfType::TimedText => postkit::mxf_wrap::EssenceType::TimedText,
        MxfType::Atmos | MxfType::DtsX => postkit::mxf_wrap::EssenceType::Atmos,
    };

    let fps = if config.frame_rate == 0 {
        24
    } else {
        config.frame_rate
    };

    let opts = postkit::mxf_wrap::MxfWrapOptions {
        input_files,
        output: config.output_mxf.clone(),
        essence_type,
        standard: postkit::mxf_wrap::MxfStandard::AsDcp,
        fps_num: fps,
        fps_den: 1,
        partition_size: 0,
        encryption: config.encryption.clone(),
    };

    let result = postkit::mxf_wrap::mxf_wrap(&opts);
    if result.success {
        tracing::info!(
            "Wrapped {:?} to MXF: {}",
            config.mxf_type,
            config.output_mxf.display()
        );
        Some(result)
    } else {
        tracing::error!("MXF wrap failed: {}", result.error);
        None
    }
}

/// Wrap essence into an MXF container using postkit's asdcplib wrapper.
pub fn wrap_mxf(config: &MxfWrapConfig) -> i32 {
    if wrap_mxf_result(config).is_some() {
        0
    } else {
        -1
    }
}
