use crate::dcp::DcpConfig;
use crate::reel::ReelRange;
use postkit::packaging::{ATMOS_DATA_TYPE_UL, DcpAuxData};
use std::path::{Path, PathBuf};

const ATMOS_HEADER_SIZE: u32 = 16384;

pub(crate) enum AtmosSource {
    FrameFiles(Vec<PathBuf>),
    TrackFile {
        path: PathBuf,
        descriptor: asdcplib::atmos::AtmosDescriptor,
    },
}

impl AtmosSource {
    pub(crate) fn open(path: &Path, fps: u32) -> Result<Self, String> {
        if !path.is_file() {
            return crate::mxf_wrap::collect_inputs(path).map(AtmosSource::FrameFiles);
        }
        let mut reader = open_track_file(path)?;
        let info = reader
            .writer_info()
            .map_err(|e| format!("cannot read the header of {}: {e}", path.display()))?;
        if info.encrypted_essence {
            return Err(format!(
                "the Atmos track file {} is encrypted: decrypt it first",
                path.display()
            ));
        }
        let descriptor = reader.atmos_descriptor().map_err(|e| {
            format!(
                "cannot read the Atmos descriptor of {}: {e}",
                path.display()
            )
        })?;
        let edit_rate = descriptor.edit_rate;
        if i64::from(edit_rate.numerator) != i64::from(fps) * i64::from(edit_rate.denominator) {
            return Err(format!(
                "the Atmos track file runs at {}/{} but the DCP is {fps}/1: its edit rate has to \
                 match the picture",
                edit_rate.numerator, edit_rate.denominator
            ));
        }
        Ok(AtmosSource::TrackFile {
            path: path.to_path_buf(),
            descriptor,
        })
    }

    pub(crate) fn open_covering(
        path: &Path,
        fps: u32,
        picture_frames: u64,
    ) -> Result<Self, String> {
        let source = Self::open(path, fps)?;
        crate::preflight::check_atmos_frame_count(source.frame_count(), picture_frames)?;
        Ok(source)
    }

    pub(crate) fn frame_count(&self) -> u64 {
        match self {
            AtmosSource::FrameFiles(files) => files.len() as u64,
            AtmosSource::TrackFile { descriptor, .. } => u64::from(descriptor.container_duration),
        }
    }
}

fn open_track_file(path: &Path) -> Result<asdcplib::atmos::MxfReader, String> {
    let mut reader = asdcplib::atmos::MxfReader::new();
    reader
        .open_read(&path.to_string_lossy())
        .map_err(|e| format!("cannot read {} as an Atmos MXF: {e}", path.display()))?;
    Ok(reader)
}

pub(crate) struct WrappedAtmosReel {
    pub(crate) uuid: String,
    pub(crate) file_name: String,
    pub(crate) path: PathBuf,
    pub(crate) aux_data: DcpAuxData,
    pub(crate) key: Option<crate::encrypt::GeneratedKey>,
}

pub(crate) fn wrap_atmos_reel(
    config: &DcpConfig,
    source: &AtmosSource,
    range: ReelRange,
    fps: u32,
) -> Result<WrappedAtmosReel, String> {
    let uuid = uuid::Uuid::new_v4();
    let file_name = format!("atmos_{uuid}.mxf");
    let path = config.output_dir.join(&file_name);
    let key = crate::versions::mint_key(config, crate::encrypt::KeyType::Mdek, &uuid.to_string())
        .map_err(|()| "minting the Atmos content key failed".to_string())?;
    let encryption = key.as_ref().map(crate::reel::mxf_enc);
    match source {
        AtmosSource::FrameFiles(files) => {
            let reel_files = files[range.start as usize..range.end as usize].to_vec();
            crate::mxf_wrap::wrap_mxf_files(
                reel_files,
                &path,
                crate::mxf_wrap::MxfType::Atmos,
                fps,
                encryption,
                None,
                Some(*uuid.as_bytes()),
            )
            .ok_or("Failed to wrap Atmos MXF")?;
        }
        AtmosSource::TrackFile {
            path: source_path,
            descriptor,
        } => rewrap_track_file_range(
            source_path,
            descriptor,
            range,
            &path,
            *uuid.as_bytes(),
            encryption,
        )?,
    }
    let aux_data = DcpAuxData {
        id: uuid.to_string(),
        edit_rate_num: fps,
        edit_rate_den: 1,
        duration: range.frames(),
        entry_point: 0,
        key_id: key.as_ref().map(|k| k.info.key_id.clone()),
        data_type: ATMOS_DATA_TYPE_UL.to_string(),
    };
    tracing::info!("Atmos MXF: {file_name} ({} frames)", range.frames());
    Ok(WrappedAtmosReel {
        uuid: uuid.to_string(),
        file_name,
        path,
        aux_data,
        key,
    })
}

fn rewrap_track_file_range(
    source_path: &Path,
    source_descriptor: &asdcplib::atmos::AtmosDescriptor,
    range: ReelRange,
    output: &Path,
    asset_uuid: [u8; 16],
    encryption: Option<postkit::mxf_wrap::MxfEncryption>,
) -> Result<(), String> {
    let mut reader = open_track_file(source_path)?;
    let mut info = asdcplib::WriterInfo {
        asset_uuid,
        context_id: *uuid::Uuid::new_v4().as_bytes(),
        label_set: asdcplib::LabelSet::Smpte,
        ..Default::default()
    };
    let mut cipher = None;
    let mut hmac = None;
    if let Some(encryption) = &encryption {
        info.encrypted_essence = true;
        info.uses_hmac = true;
        info.cryptographic_key_id = encryption.key_id;
        let mut cipher_context = asdcplib::crypto::AesEncContext::new();
        cipher_context
            .init_key(&encryption.content_key)
            .map_err(|e| format!("AES key init failed: {e}"))?;
        let mut hmac_context = asdcplib::crypto::HmacContext::new();
        hmac_context
            .init_key(&encryption.content_key, info.label_set)
            .map_err(|e| format!("HMAC key init failed: {e}"))?;
        cipher = Some(cipher_context);
        hmac = Some(hmac_context);
    }
    let descriptor = asdcplib::atmos::AtmosDescriptor {
        container_duration: range.frames() as u32,
        asset_id: asset_uuid,
        ..source_descriptor.clone()
    };
    let mut writer = asdcplib::atmos::MxfWriter::new();
    writer
        .open_write(
            &output.to_string_lossy(),
            &info,
            &descriptor,
            ATMOS_HEADER_SIZE,
        )
        .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    let mut frame = vec![0u8; crate::decrypt::MAX_FRAME_BUF];
    for frame_number in range.start..range.end {
        let length = reader
            .read_frame(frame_number as u32, &mut frame, None, None)
            .map_err(|e| {
                format!(
                    "cannot read Atmos frame {frame_number} of {}: {e}",
                    source_path.display()
                )
            })?;
        writer
            .write_frame(&frame[..length], cipher.as_mut(), hmac.as_mut())
            .map_err(|e| format!("cannot write Atmos frame {frame_number}: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("cannot finish {}: {e}", output.display()))
}
