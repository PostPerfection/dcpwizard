use serde::{Deserialize, Serialize};
use std::path::Path;

/// Metadata extracted from a DCP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcpInfo {
    pub title: String,
    pub content_kind: String,
    pub standard: String,
    pub cpl_count: usize,
    pub reel_count: usize,
    pub duration_frames: i64,
    pub frame_rate: String,
    pub encrypted: bool,
    pub assets: Vec<AssetInfo>,
}

/// Information about a single DCP asset.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetInfo {
    pub id: String,
    pub path: String,
}

/// Inspect a DCP directory and extract metadata using dcpdoctor-core.
pub fn inspect_dcp(dcp_dir: &Path) -> Result<DcpInfo, String> {
    if !dcp_dir.exists() {
        return Err(format!("DCP directory not found: {}", dcp_dir.display()));
    }

    let dcp = dcpdoctor_core::dcp::open_dcp(dcp_dir).map_err(|notes| {
        notes
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    let mut info = DcpInfo {
        standard: format!("{}", dcp.standard),
        cpl_count: dcp.cpls.len(),
        ..Default::default()
    };

    if let Some((_path, cpl)) = dcp.cpls.first() {
        info.title = cpl.content_title.clone();
        info.content_kind = cpl.content_kind.clone();
        info.reel_count = cpl.reels.len();

        if let Some(reel) = cpl.reels.first() {
            info.frame_rate = reel.picture.edit_rate.clone();
        }

        info.duration_frames = cpl.reels.iter().map(|r| r.picture.duration).sum();
    }

    for asset in &dcp.assetmap.assets {
        info.assets.push(AssetInfo {
            id: asset.id.clone(),
            path: asset.path.clone(),
        });
    }

    Ok(info)
}
