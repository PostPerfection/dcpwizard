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
    let dcp = inspect_package(dcp_dir)?;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CplIdentity {
    pub id: String,
    pub title: String,
}

pub fn cpl_identities(dcp_dir: &Path) -> Result<Vec<CplIdentity>, String> {
    let dcp = inspect_package(dcp_dir)?;
    let identities: Vec<CplIdentity> = dcp
        .cpls
        .iter()
        .map(|(_path, cpl)| CplIdentity {
            id: cpl.id.clone(),
            title: cpl.content_title.clone(),
        })
        .filter(|identity| !identity.id.is_empty())
        .collect();
    if identities.is_empty() {
        return Err(format!("no composition in {}", dcp_dir.display()));
    }
    Ok(identities)
}

fn inspect_package(dcp_dir: &Path) -> Result<dcpdoctor_core::dcp::Dcp, String> {
    if !dcp_dir.exists() {
        return Err(format!("DCP directory not found: {}", dcp_dir.display()));
    }
    dcpdoctor_core::dcp::open_dcp(dcp_dir).map_err(|notes| {
        notes
            .iter()
            .map(|note| note.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpl_identities_reads_the_composition_id_and_title() {
        let directory = tempfile::tempdir().unwrap();
        let cpl = r#"<?xml version="1.0" encoding="UTF-8"?>
<CompositionPlaylist xmlns="http://www.smpte-ra.org/schemas/429-7/2006/CPL">
  <Id>urn:uuid:96558952-39b8-42d3-825e-9ddd31298219</Id>
  <ContentTitleText>My Film</ContentTitleText>
  <ContentKind>feature</ContentKind>
</CompositionPlaylist>"#;
        std::fs::write(directory.path().join("cpl.xml"), cpl).unwrap();
        std::fs::write(
            directory.path().join("ASSETMAP.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<AssetMap xmlns="http://www.smpte-ra.org/schemas/429-9/2007/AM">
  <Id>urn:uuid:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee</Id>
  <AssetList>
    <Asset>
      <Id>urn:uuid:11111111-2222-3333-4444-555555555555</Id>
      <ChunkList><Chunk><Path>cpl.xml</Path></Chunk></ChunkList>
    </Asset>
  </AssetList>
</AssetMap>"#,
        )
        .unwrap();

        let identities = cpl_identities(directory.path()).unwrap();

        assert_eq!(
            identities,
            vec![CplIdentity {
                id: "96558952-39b8-42d3-825e-9ddd31298219".into(),
                title: "My Film".into(),
            }]
        );
    }
}
