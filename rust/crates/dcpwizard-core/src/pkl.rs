use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// PKL entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PklEntry {
    pub id: String,
    pub asset_type: String,
    pub file: PathBuf,
    pub hash: String,
    pub size: u64,
}

/// Generate a Packing List XML via the shared postkit writer.
pub fn generate_pkl(
    entries: &[PklEntry],
    pkl_uuid: &str,
    standard: crate::Standard,
    output_file: &Path,
) -> i32 {
    use postkit::packaging::{self, PackingList, PklAsset};

    let namespace = match standard {
        crate::Standard::Smpte => packaging::ns::PKL_SMPTE,
        crate::Standard::Interop => packaging::ns::PKL_INTEROP,
    };

    let assets = entries
        .iter()
        .map(|e| PklAsset {
            id: e.id.clone(),
            hash: e.hash.clone(),
            size: e.size,
            asset_type: e.asset_type.clone(),
        })
        .collect();

    let pkl = PackingList {
        uuid: pkl_uuid.to_string(),
        namespace: namespace.to_string(),
        issuer: "DCP Wizard".into(),
        creator: "DCP Wizard".into(),
        issue_date: chrono::Utc::now().to_rfc3339(),
        assets,
    };

    match std::fs::write(output_file, pkl.to_xml()) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("Failed to write PKL: {e}");
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_pkl_uses_supplied_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("PKL_expected.xml");

        assert_eq!(
            generate_pkl(&[], "expected", crate::Standard::Smpte, &path),
            0
        );
        let xml = std::fs::read_to_string(path).unwrap();
        assert!(xml.contains("<Id>urn:uuid:expected</Id>"));
        assert!(xml.contains("<IssueDate>"));
    }
}
