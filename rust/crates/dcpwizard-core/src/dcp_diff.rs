//! Compare two DCPs and report differences in tracks, CPL metadata, and assets.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffError {
    #[error("DCP directory does not exist: {0}")]
    NotFound(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Removed,
    Modified,
    Unchanged,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackDiff {
    pub track_id: String,
    pub essence_type: String,
    pub status: DiffStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReelDiff {
    pub cpl_id: String,
    pub reel_index: u32,
    pub status: DiffStatus,
    pub old_asset_id: String,
    pub new_asset_id: String,
}

#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub dcp_a: PathBuf,
    pub dcp_b: PathBuf,
    pub include_hashes: bool,
    pub show_unchanged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub tracks_added: u32,
    pub tracks_removed: u32,
    pub tracks_modified: u32,
    pub reels_changed: u32,
    pub track_diffs: Vec<TrackDiff>,
    pub reel_diffs: Vec<ReelDiff>,
    pub cpl_title_changed: bool,
    pub cpl_annotation_changed: bool,
    pub edit_rate_changed: bool,
}

#[derive(Debug, Clone)]
struct AssetInfo {
    #[allow(dead_code)]
    id: String,
    asset_type: String,
    size: u64,
    path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct CplInfo {
    id: String,
    title: String,
    annotation: String,
    edit_rate: String,
    asset_ids: Vec<String>,
}

fn parse_assetmap(dcp_dir: &Path) -> HashMap<String, AssetInfo> {
    let mut assets = HashMap::new();

    let assetmap = if dcp_dir.join("ASSETMAP.xml").exists() {
        dcp_dir.join("ASSETMAP.xml")
    } else if dcp_dir.join("ASSETMAP").exists() {
        dcp_dir.join("ASSETMAP")
    } else {
        return assets;
    };

    let Ok(content) = fs::read_to_string(&assetmap) else {
        return assets;
    };

    for asset_block in content.split("<Asset>").skip(1) {
        let Some(end) = asset_block.find("</Asset>") else {
            continue;
        };
        let block = &asset_block[..end];

        let id = extract_tag(block, "Id")
            .unwrap_or_default()
            .trim_start_matches("urn:uuid:")
            .to_string();

        let rel_path = extract_tag(block, "Path").unwrap_or_default();

        if id.is_empty() || rel_path.is_empty() {
            continue;
        }

        let full_path = dcp_dir.join(&rel_path);
        let size = fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);

        let ext = full_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let asset_type = match ext.as_str() {
            "mxf" => "mxf",
            "xml" => "xml",
            _ => "other",
        }
        .to_string();

        assets.insert(
            id.clone(),
            AssetInfo {
                id,
                asset_type,
                size,
                path: full_path,
            },
        );
    }

    assets
}

fn parse_cpl(cpl_path: &Path) -> CplInfo {
    let mut info = CplInfo::default();

    let Ok(content) = fs::read_to_string(cpl_path) else {
        return info;
    };

    info.id = extract_tag(&content, "Id")
        .unwrap_or_default()
        .trim_start_matches("urn:uuid:")
        .to_string();
    info.title = extract_tag(&content, "ContentTitleText")
        .or_else(|| extract_tag(&content, "ContentTitle"))
        .unwrap_or_default();
    info.annotation = extract_tag(&content, "AnnotationText")
        .or_else(|| extract_tag(&content, "Annotation"))
        .unwrap_or_default();
    info.edit_rate = extract_tag(&content, "EditRate").unwrap_or_default();

    // Extract all asset IDs from MainPicture/MainSound Id elements
    for tag in ["Id"] {
        for segment in content.split(&format!("<{tag}>")).skip(1) {
            if let Some(end) = segment.find(&format!("</{tag}>")) {
                let raw = segment[..end].trim();
                let id = raw.trim_start_matches("urn:uuid:").to_string();
                if !id.is_empty() && id != info.id {
                    info.asset_ids.push(id);
                }
            }
        }
    }

    info
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

fn find_cpl(assets: &HashMap<String, AssetInfo>) -> Option<&AssetInfo> {
    assets.values().find(|a| {
        a.asset_type == "xml"
            && a.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("CPL"))
                .unwrap_or(false)
    })
}

/// Compare two DCPs and return a detailed diff.
pub fn diff_packages(opts: &DiffOptions) -> Result<DiffResult, DiffError> {
    if !opts.dcp_a.exists() {
        return Err(DiffError::NotFound(opts.dcp_a.clone()));
    }
    if !opts.dcp_b.exists() {
        return Err(DiffError::NotFound(opts.dcp_b.clone()));
    }

    let assets_a = parse_assetmap(&opts.dcp_a);
    let assets_b = parse_assetmap(&opts.dcp_b);

    let cpl_info_a = find_cpl(&assets_a)
        .map(|a| parse_cpl(&a.path))
        .unwrap_or_default();
    let cpl_info_b = find_cpl(&assets_b)
        .map(|a| parse_cpl(&a.path))
        .unwrap_or_default();

    let mut result = DiffResult {
        tracks_added: 0,
        tracks_removed: 0,
        tracks_modified: 0,
        reels_changed: 0,
        track_diffs: Vec::new(),
        reel_diffs: Vec::new(),
        cpl_title_changed: cpl_info_a.title != cpl_info_b.title,
        cpl_annotation_changed: cpl_info_a.annotation != cpl_info_b.annotation,
        edit_rate_changed: cpl_info_a.edit_rate != cpl_info_b.edit_rate,
    };

    // Compare MXF tracks
    for (id, asset) in &assets_a {
        if asset.asset_type != "mxf" {
            continue;
        }

        let status;
        let detail;

        if let Some(other) = assets_b.get(id) {
            if opts.include_hashes && asset.size != other.size {
                status = DiffStatus::Modified;
                detail = format!("Track {id} size changed ({} → {})", asset.size, other.size);
                result.tracks_modified += 1;
            } else {
                status = DiffStatus::Unchanged;
                detail = String::new();
                if !opts.show_unchanged {
                    continue;
                }
            }
        } else {
            status = DiffStatus::Removed;
            detail = format!("Track {id} removed in B");
            result.tracks_removed += 1;
        }

        result.track_diffs.push(TrackDiff {
            track_id: id.clone(),
            essence_type: "video".to_string(),
            status,
            detail,
        });
    }

    // Find added tracks
    for (id, asset) in &assets_b {
        if asset.asset_type != "mxf" {
            continue;
        }
        if !assets_a.contains_key(id) {
            result.tracks_added += 1;
            result.track_diffs.push(TrackDiff {
                track_id: id.clone(),
                essence_type: "video".to_string(),
                status: DiffStatus::Added,
                detail: format!("New track {id} in B"),
            });
        }
    }

    // Compare asset references in CPL
    let refs_a: HashSet<&str> = cpl_info_a.asset_ids.iter().map(|s| s.as_str()).collect();
    let refs_b: HashSet<&str> = cpl_info_b.asset_ids.iter().map(|s| s.as_str()).collect();

    let mut reel_idx = 0u32;
    for aid in &refs_a {
        if !refs_b.contains(aid) {
            result.reels_changed += 1;
            result.reel_diffs.push(ReelDiff {
                cpl_id: cpl_info_a.id.clone(),
                reel_index: reel_idx,
                status: DiffStatus::Removed,
                old_asset_id: aid.to_string(),
                new_asset_id: String::new(),
            });
        }
        reel_idx += 1;
    }
    for aid in &refs_b {
        if !refs_a.contains(aid) {
            result.reels_changed += 1;
            result.reel_diffs.push(ReelDiff {
                cpl_id: cpl_info_b.id.clone(),
                reel_index: reel_idx,
                status: DiffStatus::Added,
                old_asset_id: String::new(),
                new_asset_id: aid.to_string(),
            });
            reel_idx += 1;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dcp(dir: &Path, cpl_title: &str, track_ids: &[&str]) {
        let mut asset_entries = String::new();
        let cpl_filename = "CPL_test.xml";

        asset_entries.push_str(&format!(
            r#"<Asset><Id>urn:uuid:cpl-id-1234</Id><ChunkList><Chunk><Path>{}</Path></Chunk></ChunkList></Asset>"#,
            cpl_filename
        ));

        for tid in track_ids {
            let filename = format!("{}.mxf", tid);
            asset_entries.push_str(&format!(
                r#"<Asset><Id>urn:uuid:{}</Id><ChunkList><Chunk><Path>{}</Path></Chunk></ChunkList></Asset>"#,
                tid, filename
            ));
            fs::write(dir.join(&filename), vec![0u8; 1024]).unwrap();
        }

        let assetmap = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<AssetMap xmlns="http://www.smpte-ra.org/schemas/429-9/2007/AM">
  <Id>urn:uuid:am-1234</Id>
  <AssetList>{}</AssetList>
</AssetMap>"#,
            asset_entries
        );
        fs::write(dir.join("ASSETMAP.xml"), assetmap).unwrap();

        let track_refs: String = track_ids
            .iter()
            .map(|tid| format!("<Id>urn:uuid:{}</Id>", tid))
            .collect::<Vec<_>>()
            .join("\n");

        let cpl = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CompositionPlaylist xmlns="http://www.smpte-ra.org/schemas/429-7/2006/CPL">
  <Id>urn:uuid:cpl-id-1234</Id>
  <ContentTitleText>{}</ContentTitleText>
  <AnnotationText>Test annotation</AnnotationText>
  <EditRate>24 1</EditRate>
  <ReelList>
    <Reel>
      <AssetList>
        {}
      </AssetList>
    </Reel>
  </ReelList>
</CompositionPlaylist>"#,
            cpl_title, track_refs
        );
        fs::write(dir.join(cpl_filename), cpl).unwrap();
    }

    #[test]
    fn test_identical_packages() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        create_test_dcp(tmp_a.path(), "Test Film", &["track-001", "track-002"]);
        create_test_dcp(tmp_b.path(), "Test Film", &["track-001", "track-002"]);

        let opts = DiffOptions {
            dcp_a: tmp_a.path().to_path_buf(),
            dcp_b: tmp_b.path().to_path_buf(),
            include_hashes: false,
            show_unchanged: false,
        };

        let result = diff_packages(&opts).unwrap();
        assert_eq!(result.tracks_added, 0);
        assert_eq!(result.tracks_removed, 0);
        assert_eq!(result.tracks_modified, 0);
        assert_eq!(result.reels_changed, 0);
        assert!(!result.cpl_title_changed);
        assert!(!result.edit_rate_changed);
    }

    #[test]
    fn test_track_added() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        create_test_dcp(tmp_a.path(), "Test Film", &["track-001"]);
        create_test_dcp(tmp_b.path(), "Test Film", &["track-001", "track-002"]);

        let opts = DiffOptions {
            dcp_a: tmp_a.path().to_path_buf(),
            dcp_b: tmp_b.path().to_path_buf(),
            include_hashes: false,
            show_unchanged: false,
        };

        let result = diff_packages(&opts).unwrap();
        assert_eq!(result.tracks_added, 1);
        assert_eq!(result.tracks_removed, 0);
    }

    #[test]
    fn test_track_removed() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        create_test_dcp(tmp_a.path(), "Test Film", &["track-001", "track-002"]);
        create_test_dcp(tmp_b.path(), "Test Film", &["track-001"]);

        let opts = DiffOptions {
            dcp_a: tmp_a.path().to_path_buf(),
            dcp_b: tmp_b.path().to_path_buf(),
            include_hashes: false,
            show_unchanged: false,
        };

        let result = diff_packages(&opts).unwrap();
        assert_eq!(result.tracks_removed, 1);
        assert_eq!(result.tracks_added, 0);
    }

    #[test]
    fn test_title_changed() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        create_test_dcp(tmp_a.path(), "Version 1", &["track-001"]);
        create_test_dcp(tmp_b.path(), "Version 2", &["track-001"]);

        let opts = DiffOptions {
            dcp_a: tmp_a.path().to_path_buf(),
            dcp_b: tmp_b.path().to_path_buf(),
            include_hashes: false,
            show_unchanged: false,
        };

        let result = diff_packages(&opts).unwrap();
        assert!(result.cpl_title_changed);
        assert_eq!(result.tracks_added, 0);
        assert_eq!(result.tracks_removed, 0);
    }

    #[test]
    fn test_size_change_detected_with_hashes() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        create_test_dcp(tmp_a.path(), "Test Film", &["track-001"]);
        create_test_dcp(tmp_b.path(), "Test Film", &["track-001"]);

        // Modify file size in B
        fs::write(tmp_b.path().join("track-001.mxf"), vec![0u8; 2048]).unwrap();

        let opts = DiffOptions {
            dcp_a: tmp_a.path().to_path_buf(),
            dcp_b: tmp_b.path().to_path_buf(),
            include_hashes: true,
            show_unchanged: false,
        };

        let result = diff_packages(&opts).unwrap();
        assert_eq!(result.tracks_modified, 1);
    }

    #[test]
    fn test_missing_dcp_returns_error() {
        let tmp = TempDir::new().unwrap();
        create_test_dcp(tmp.path(), "Test", &["t1"]);

        let opts = DiffOptions {
            dcp_a: tmp.path().to_path_buf(),
            dcp_b: PathBuf::from("/nonexistent/path"),
            include_hashes: false,
            show_unchanged: false,
        };

        let result = diff_packages(&opts);
        assert!(result.is_err());
    }

    #[test]
    fn test_show_unchanged() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        create_test_dcp(tmp_a.path(), "Test", &["track-001", "track-002"]);
        create_test_dcp(tmp_b.path(), "Test", &["track-001", "track-002"]);

        let opts = DiffOptions {
            dcp_a: tmp_a.path().to_path_buf(),
            dcp_b: tmp_b.path().to_path_buf(),
            include_hashes: false,
            show_unchanged: true,
        };

        let result = diff_packages(&opts).unwrap();
        assert_eq!(result.track_diffs.len(), 2);
        assert!(
            result
                .track_diffs
                .iter()
                .all(|d| d.status == DiffStatus::Unchanged)
        );
    }
}
