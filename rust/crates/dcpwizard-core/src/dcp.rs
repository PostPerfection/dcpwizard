use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// DCP creation configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcpConfig {
    pub title: String,
    pub standard: crate::Standard,
    pub resolution: crate::Resolution,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub max_bitrate_mbps: u32,
    pub encrypt: bool,
    pub stereo_3d: bool,
    pub output_dir: PathBuf,
    /// Directory containing J2K frames to wrap into picture MXF
    pub j2k_dir: Option<PathBuf>,
    /// Audio WAV file to wrap into sound MXF
    pub audio_path: Option<PathBuf>,
}

/// Create a complete DCP from the given configuration.
///
/// This orchestrates the full DCP creation pipeline:
/// 1. Wrap J2K frames and audio into MXF
/// 2. Generate CPL, PKL, ASSETMAP
/// 3. Optionally encrypt
pub fn create_dcp(config: &DcpConfig) -> i32 {
    tracing::info!(
        "Creating DCP: {} ({})",
        config.title,
        if config.standard == crate::Standard::Smpte {
            "SMPTE"
        } else {
            "Interop"
        }
    );

    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let fps = if config.frame_rate_num > 0 {
        config.frame_rate_num
    } else {
        24
    };

    // ── Wrap picture MXF ──────────────────────────────────────────────
    let picture_uuid = uuid::Uuid::new_v4().to_string();
    let picture_mxf_name = format!("picture_{picture_uuid}.mxf");
    let picture_mxf_path = config.output_dir.join(&picture_mxf_name);
    let mut picture_duration: u64 = 0;

    if let Some(ref j2k_dir) = config.j2k_dir {
        // Count frames for duration
        if let Ok(entries) = std::fs::read_dir(j2k_dir) {
            picture_duration = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let ext = e
                        .path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.to_lowercase())
                        .unwrap_or_default();
                    ext == "j2c" || ext == "j2k"
                })
                .count() as u64;
        }

        let wrap_config = crate::mxf_wrap::MxfWrapConfig {
            input_path: j2k_dir.clone(),
            output_mxf: picture_mxf_path.clone(),
            mxf_type: crate::mxf_wrap::MxfType::J2kPicture,
            frame_rate: fps,
        };
        if crate::mxf_wrap::wrap_mxf(&wrap_config) != 0 {
            tracing::error!("Failed to wrap picture MXF");
            return -1;
        }
        tracing::info!(
            "Picture MXF: {} ({picture_duration} frames)",
            picture_mxf_name
        );
    }

    // ── Wrap sound MXF ────────────────────────────────────────────────
    let sound_uuid = uuid::Uuid::new_v4().to_string();
    let sound_mxf_name = format!("sound_{sound_uuid}.mxf");
    let sound_mxf_path = config.output_dir.join(&sound_mxf_name);
    let mut has_sound = false;
    let sound_duration = picture_duration; // match picture duration

    if let Some(ref audio_path) = config.audio_path
        && audio_path.exists()
    {
        let wrap_config = crate::mxf_wrap::MxfWrapConfig {
            input_path: audio_path.clone(),
            output_mxf: sound_mxf_path.clone(),
            mxf_type: crate::mxf_wrap::MxfType::PcmAudio,
            frame_rate: fps,
        };
        if crate::mxf_wrap::wrap_mxf(&wrap_config) != 0 {
            tracing::error!("Failed to wrap sound MXF");
            return -1;
        }
        has_sound = true;
        tracing::info!("Sound MXF: {sound_mxf_name}");
    }

    // ── Generate CPL ──────────────────────────────────────────────────
    let cpl_uuid = uuid::Uuid::new_v4().to_string();
    let pkl_uuid = uuid::Uuid::new_v4().to_string();

    let reel = crate::cpl::CplReel {
        reel_id: uuid::Uuid::new_v4().to_string(),
        picture_id: picture_uuid.clone(),
        picture_edit_rate_num: fps,
        picture_edit_rate_den: 1,
        picture_duration,
        picture_entry_point: 0,
        sound_id: if has_sound {
            Some(sound_uuid.clone())
        } else {
            None
        },
        sound_edit_rate_num: fps,
        sound_edit_rate_den: 1,
        sound_duration,
        sound_entry_point: 0,
    };

    let cpl_path = config.output_dir.join(format!("CPL_{cpl_uuid}.xml"));
    let cpl_config = crate::cpl::CplConfig {
        title: config.title.clone(),
        content_kind: "feature".into(),
        reels: vec![reel],
        ..Default::default()
    };
    if crate::cpl::generate_cpl(&cpl_config, &cpl_path) != 0 {
        tracing::error!("Failed to generate CPL");
        return -1;
    }

    // ── Generate PKL ──────────────────────────────────────────────────
    let pkl_path = config.output_dir.join(format!("PKL_{pkl_uuid}.xml"));
    let cpl_hash = crate::hash::hash_file(&cpl_path).unwrap_or_default();
    let cpl_size = std::fs::metadata(&cpl_path).map(|m| m.len()).unwrap_or(0);
    let mut pkl_entries = vec![crate::pkl::PklEntry {
        id: cpl_uuid.clone(),
        asset_type: "text/xml".into(),
        file: cpl_path.clone(),
        hash: cpl_hash,
        size: cpl_size,
    }];

    if config.j2k_dir.is_some() {
        let pic_hash = crate::hash::hash_file(&picture_mxf_path).unwrap_or_default();
        let pic_size = std::fs::metadata(&picture_mxf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        pkl_entries.push(crate::pkl::PklEntry {
            id: picture_uuid.clone(),
            asset_type: "application/mxf".into(),
            file: picture_mxf_path.clone(),
            hash: pic_hash,
            size: pic_size,
        });
    }
    if has_sound {
        let snd_hash = crate::hash::hash_file(&sound_mxf_path).unwrap_or_default();
        let snd_size = std::fs::metadata(&sound_mxf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        pkl_entries.push(crate::pkl::PklEntry {
            id: sound_uuid.clone(),
            asset_type: "application/mxf".into(),
            file: sound_mxf_path.clone(),
            hash: snd_hash,
            size: snd_size,
        });
    }

    if crate::pkl::generate_pkl(&pkl_entries, &pkl_path) != 0 {
        tracing::error!("Failed to generate PKL");
        return -1;
    }

    // ── Generate ASSETMAP ─────────────────────────────────────────────
    let mut am_entries = vec![
        crate::assetmap::AssetMapEntry {
            id: pkl_uuid,
            path: pkl_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        },
        crate::assetmap::AssetMapEntry {
            id: cpl_uuid,
            path: cpl_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        },
    ];
    if config.j2k_dir.is_some() {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: picture_uuid,
            path: picture_mxf_name,
        });
    }
    if has_sound {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: sound_uuid,
            path: sound_mxf_name,
        });
    }

    if crate::assetmap::generate_assetmap(&am_entries, &config.output_dir) != 0 {
        tracing::error!("Failed to generate ASSETMAP");
        return -1;
    }

    // Encrypt if requested
    if config.encrypt {
        let enc_config = crate::encrypt::EncryptionConfig {
            dcp_dir: config.output_dir.clone(),
            ..Default::default()
        };
        if crate::encrypt::encrypt_dcp(&enc_config) != 0 {
            tracing::error!("Encryption failed");
            return -1;
        }
    }

    tracing::info!("DCP created: {}", config.output_dir.display());
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dcp() {
        let dir = tempfile::tempdir().unwrap();
        let config = DcpConfig {
            title: "Test Film".into(),
            output_dir: dir.path().to_path_buf(),
            frame_rate_num: 24,
            frame_rate_den: 1,
            ..Default::default()
        };
        assert_eq!(create_dcp(&config), 0);
        assert!(dir.path().join("ASSETMAP.xml").exists());
    }
}
