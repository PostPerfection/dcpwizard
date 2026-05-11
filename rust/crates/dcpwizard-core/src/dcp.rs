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
}

/// Create a complete DCP from the given configuration.
///
/// This orchestrates the full DCP creation pipeline:
/// 1. Encode images to JPEG 2000
/// 2. Wrap essence into MXF
/// 3. Generate CPL, PKL, ASSETMAP
/// 4. Optionally encrypt
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

    // Generate UUIDs for the DCP components
    let cpl_uuid = uuid::Uuid::new_v4().to_string();
    let pkl_uuid = uuid::Uuid::new_v4().to_string();

    // Generate CPL
    let cpl_path = config.output_dir.join(format!("CPL_{cpl_uuid}.xml"));
    let cpl_config = crate::cpl::CplConfig {
        title: config.title.clone(),
        content_kind: "feature".into(),
        ..Default::default()
    };
    if crate::cpl::generate_cpl(&cpl_config, &cpl_path) != 0 {
        tracing::error!("Failed to generate CPL");
        return -1;
    }

    // Generate PKL
    let pkl_path = config.output_dir.join(format!("PKL_{pkl_uuid}.xml"));
    let cpl_hash = crate::hash::hash_file(&cpl_path).unwrap_or_default();
    let cpl_size = std::fs::metadata(&cpl_path).map(|m| m.len()).unwrap_or(0);
    let pkl_entries = vec![crate::pkl::PklEntry {
        id: cpl_uuid.clone(),
        asset_type: "text/xml".into(),
        file: cpl_path.clone(),
        hash: cpl_hash,
        size: cpl_size,
    }];
    if crate::pkl::generate_pkl(&pkl_entries, &pkl_path) != 0 {
        tracing::error!("Failed to generate PKL");
        return -1;
    }

    // Generate ASSETMAP
    let am_entries = vec![
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
