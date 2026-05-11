use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Audio channel layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChannelLayout {
    Mono,
    #[default]
    Stereo,
    FiveOne,
    SevenOne,
    Atmos,
}

impl ChannelLayout {
    pub fn channel_count(&self) -> u32 {
        match self {
            ChannelLayout::Mono => 1,
            ChannelLayout::Stereo => 2,
            ChannelLayout::FiveOne => 6,
            ChannelLayout::SevenOne => 8,
            ChannelLayout::Atmos => 16,
        }
    }
}

/// Audio wrapping configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioConfig {
    pub input_file: PathBuf,
    pub output_mxf: PathBuf,
    pub layout: ChannelLayout,
    pub sample_rate: u32,
    pub bit_depth: u32,
}

/// Wrap audio into DCP-compliant MXF via PCM WAV intermediate.
///
/// Pipeline: input → ffmpeg (PCM WAV 24-bit 48kHz) → asdcp-wrap (MXF).
pub fn wrap_audio(config: &AudioConfig) -> i32 {
    let sample_rate = if config.sample_rate == 0 {
        48000
    } else {
        config.sample_rate
    };
    let bit_depth = if config.bit_depth == 0 {
        24
    } else {
        config.bit_depth
    };
    let channels = config.layout.channel_count();

    // Step 1: Convert to PCM WAV
    let wav_path = config.output_mxf.with_extension("wav");
    let pcm_format = match bit_depth {
        16 => "pcm_s16le",
        32 => "pcm_s32le",
        _ => "pcm_s24le",
    };

    let ffmpeg_result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&config.input_file)
        .arg("-acodec")
        .arg(pcm_format)
        .arg("-ar")
        .arg(sample_rate.to_string())
        .arg("-ac")
        .arg(channels.to_string())
        .arg(&wav_path)
        .output();

    match &ffmpeg_result {
        Ok(o) if o.status.success() => {
            tracing::info!("Converted audio to WAV: {}", wav_path.display());
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg audio conversion failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            return -1;
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            return -1;
        }
    }

    // Step 2: Wrap WAV into MXF via asdcp-wrap
    let asdcp_result = std::process::Command::new("asdcp-wrap")
        .arg("-a")
        .arg(&wav_path)
        .arg(&config.output_mxf)
        .output();

    // Clean up intermediate WAV
    let _ = std::fs::remove_file(&wav_path);

    match asdcp_result {
        Ok(o) if o.status.success() => {
            tracing::info!("Wrapped audio MXF: {}", config.output_mxf.display());
            0
        }
        Ok(o) => {
            tracing::error!(
                "asdcp-wrap audio failed: {}",
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
