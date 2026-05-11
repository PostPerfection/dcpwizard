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

fn make_writer_info() -> asdcplib::WriterInfo {
    let asset_uuid = uuid::Uuid::new_v4();
    let context_id = uuid::Uuid::new_v4();
    asdcplib::WriterInfo {
        asset_uuid: *asset_uuid.as_bytes(),
        context_id: *context_id.as_bytes(),
        label_set: asdcplib::LabelSet::Smpte,
        ..Default::default()
    }
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

/// Wrap essence into an MXF container using asdcplib FFI.
pub fn wrap_mxf(config: &MxfWrapConfig) -> i32 {
    match config.mxf_type {
        MxfType::J2kPicture => wrap_j2k(config),
        MxfType::PcmAudio => wrap_pcm(config),
        MxfType::TimedText => wrap_timed_text(config),
        MxfType::Atmos | MxfType::DtsX => wrap_atmos(config),
    }
}

fn wrap_j2k(config: &MxfWrapConfig) -> i32 {
    let files = match collect_inputs(&config.input_path) {
        Ok(f) => f,
        Err(e) => { tracing::error!("{e}"); return -1; }
    };

    let mut frames: Vec<Vec<u8>> = Vec::new();
    for f in &files {
        match std::fs::read(f) {
            Ok(d) => frames.push(d),
            Err(e) => { tracing::error!("read {}: {e}", f.display()); return -1; }
        }
    }

    let fps = if config.frame_rate == 0 { 24 } else { config.frame_rate };
    let info = make_writer_info();
    let desc = asdcplib::jp2k::PictureDescriptor {
        edit_rate: asdcplib::Rational::new(fps as i32, 1),
        sample_rate: asdcplib::Rational::new(fps as i32, 1),
        stored_width: 2048,
        stored_height: 1080,
        aspect_ratio: asdcplib::Rational::new(2048, 1080),
        container_duration: frames.len() as u32,
        component_count: 3,
    };

    let mut writer = asdcplib::jp2k::MxfWriter::new();
    let output_str = config.output_mxf.to_string_lossy().to_string();
    if let Err(e) = writer.open_write(&output_str, &info, &desc, 16384) {
        tracing::error!("JP2K open_write: {e}");
        return -1;
    }

    for frame in &frames {
        if let Err(e) = writer.write_frame(frame, None, None) {
            tracing::error!("JP2K write_frame: {e}");
            return -1;
        }
    }

    if let Err(e) = writer.finalize() {
        tracing::error!("JP2K finalize: {e}");
        return -1;
    }

    tracing::info!("Wrapped {:?} to MXF: {}", config.mxf_type, config.output_mxf.display());
    0
}

fn wrap_pcm(config: &MxfWrapConfig) -> i32 {
    let files = match collect_inputs(&config.input_path) {
        Ok(f) => f,
        Err(e) => { tracing::error!("{e}"); return -1; }
    };

    let wav_data = match std::fs::read(&files[0]) {
        Ok(d) => d,
        Err(e) => { tracing::error!("read WAV: {e}"); return -1; }
    };

    let fps = if config.frame_rate == 0 { 24 } else { config.frame_rate };
    let info = make_writer_info();
    let channels = 6u32;
    let bits = 24u32;
    let sample_rate = 48000u32;
    let block_align = (bits / 8) * channels;
    let samples_per_frame = (sample_rate as f64 / fps as f64).ceil() as u32;
    let frame_size = samples_per_frame * block_align;

    let pcm_start = if wav_data.len() > 44 { 44 } else { 0 };
    let pcm_data = &wav_data[pcm_start..];
    let num_frames = if frame_size > 0 { pcm_data.len() as u32 / frame_size } else { 0 };

    let desc = asdcplib::pcm::AudioDescriptor {
        edit_rate: asdcplib::Rational::new(fps as i32, 1),
        audio_sampling_rate: asdcplib::Rational::new(sample_rate as i32, 1),
        locked: true,
        channel_count: channels,
        quantization_bits: bits,
        block_align,
        avg_bps: sample_rate * block_align,
        linked_track_id: 0,
        container_duration: num_frames,
        channel_format: asdcplib::pcm::ChannelFormat::Cfg1,
    };

    let mut writer = asdcplib::pcm::MxfWriter::new();
    let output_str = config.output_mxf.to_string_lossy().to_string();
    if let Err(e) = writer.open_write(&output_str, &info, &desc, 16384) {
        tracing::error!("PCM open_write: {e}");
        return -1;
    }

    for i in 0..num_frames {
        let start = (i * frame_size) as usize;
        let end = start + frame_size as usize;
        if end > pcm_data.len() { break; }
        if let Err(e) = writer.write_frame(&pcm_data[start..end], None, None) {
            tracing::error!("PCM write_frame: {e}");
            return -1;
        }
    }

    if let Err(e) = writer.finalize() {
        tracing::error!("PCM finalize: {e}");
        return -1;
    }

    tracing::info!("Wrapped {:?} to MXF: {}", config.mxf_type, config.output_mxf.display());
    0
}

fn wrap_timed_text(config: &MxfWrapConfig) -> i32 {
    let files = match collect_inputs(&config.input_path) {
        Ok(f) => f,
        Err(e) => { tracing::error!("{e}"); return -1; }
    };

    let xml_data = match std::fs::read_to_string(&files[0]) {
        Ok(d) => d,
        Err(e) => { tracing::error!("read XML: {e}"); return -1; }
    };

    let fps = if config.frame_rate == 0 { 24 } else { config.frame_rate };
    let info = make_writer_info();
    let desc = asdcplib::timed_text::TimedTextDescriptor {
        edit_rate: asdcplib::Rational::new(fps as i32, 1),
        container_duration: fps * 60,
        asset_id: info.asset_uuid,
    };

    let mut writer = asdcplib::timed_text::MxfWriter::new();
    let output_str = config.output_mxf.to_string_lossy().to_string();
    if let Err(e) = writer.open_write(&output_str, &info, &desc, 16384) {
        tracing::error!("TimedText open_write: {e}");
        return -1;
    }

    if let Err(e) = writer.write_timed_text_resource(&xml_data, None, None) {
        tracing::error!("TimedText write: {e}");
        return -1;
    }

    for f in files.iter().skip(1) {
        let data = match std::fs::read(f) {
            Ok(d) => d,
            Err(e) => { tracing::error!("read {}: {e}", f.display()); return -1; }
        };
        let resource_uuid = *uuid::Uuid::new_v4().as_bytes();
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let mime = match ext.as_str() {
            "ttf" | "otf" => "application/x-font-opentype",
            "png" => "image/png",
            _ => "application/octet-stream",
        };
        if let Err(e) = writer.write_ancillary_resource(&data, &resource_uuid, mime, None, None) {
            tracing::error!("TimedText ancillary: {e}");
            return -1;
        }
    }

    if let Err(e) = writer.finalize() {
        tracing::error!("TimedText finalize: {e}");
        return -1;
    }

    tracing::info!("Wrapped {:?} to MXF: {}", config.mxf_type, config.output_mxf.display());
    0
}

fn wrap_atmos(config: &MxfWrapConfig) -> i32 {
    let files = match collect_inputs(&config.input_path) {
        Ok(f) => f,
        Err(e) => { tracing::error!("{e}"); return -1; }
    };

    let mut frames: Vec<Vec<u8>> = Vec::new();
    for f in &files {
        match std::fs::read(f) {
            Ok(d) => frames.push(d),
            Err(e) => { tracing::error!("read {}: {e}", f.display()); return -1; }
        }
    }

    let fps = if config.frame_rate == 0 { 24 } else { config.frame_rate };
    let info = make_writer_info();
    let desc = asdcplib::atmos::AtmosDescriptor {
        edit_rate: asdcplib::Rational::new(fps as i32, 1),
        container_duration: frames.len() as u32,
        asset_id: info.asset_uuid,
        data_essence_coding: [0; 16],
        first_frame: 0,
        max_channel_count: 128,
        max_object_count: 118,
        atmos_id: *uuid::Uuid::new_v4().as_bytes(),
        atmos_version: 1,
    };

    let mut writer = asdcplib::atmos::MxfWriter::new();
    let output_str = config.output_mxf.to_string_lossy().to_string();
    if let Err(e) = writer.open_write(&output_str, &info, &desc, 16384) {
        tracing::error!("Atmos open_write: {e}");
        return -1;
    }

    for frame in &frames {
        if let Err(e) = writer.write_frame(frame, None, None) {
            tracing::error!("Atmos write_frame: {e}");
            return -1;
        }
    }

    if let Err(e) = writer.finalize() {
        tracing::error!("Atmos finalize: {e}");
        return -1;
    }

    tracing::info!("Wrapped {:?} to MXF: {}", config.mxf_type, config.output_mxf.display());
    0
}
