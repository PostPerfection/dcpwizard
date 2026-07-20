use serde::{Deserialize, Serialize};
use std::path::Path;

/// ASSETMAP entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetMapEntry {
    pub id: String,
    pub path: String,
    pub packing_list: bool,
}

/// Generate an ASSETMAP and VOLINDEX for the DCP.
pub fn generate_assetmap(
    entries: &[AssetMapEntry],
    output_dir: &Path,
    standard: crate::Standard,
) -> i32 {
    use postkit::packaging::{self, AssetMap, AssetMapAsset, volindex_xml};

    let (namespace, assetmap_name, volindex_name) = match standard {
        crate::Standard::Smpte => (packaging::ns::AM_SMPTE, "ASSETMAP.xml", "VOLINDEX.xml"),
        crate::Standard::Interop => (packaging::ns::AM_INTEROP, "ASSETMAP", "VOLINDEX"),
    };

    let assets = entries
        .iter()
        .map(|e| AssetMapAsset {
            id: e.id.clone(),
            path: e.path.clone(),
            packing_list: e.packing_list,
        })
        .collect();

    let am = AssetMap {
        uuid: uuid::Uuid::new_v4().to_string(),
        namespace: namespace.to_string(),
        creator: "DCP Wizard".into(),
        include_volume_count: true,
        assets,
    };

    let am_path = output_dir.join(assetmap_name);
    if let Err(e) = std::fs::write(&am_path, am.to_xml()) {
        tracing::error!("Failed to write ASSETMAP: {e}");
        return -1;
    }

    let vi_path = output_dir.join(volindex_name);
    if let Err(e) = std::fs::write(&vi_path, volindex_xml(namespace)) {
        tracing::error!("Failed to write VOLINDEX: {e}");
        return -1;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_interop_assetmap_marks_packing_list() {
        let dir = tempfile::tempdir().unwrap();
        let entries = [AssetMapEntry {
            id: "pkl-id".into(),
            path: "PKL.xml".into(),
            packing_list: true,
        }];

        assert_eq!(
            generate_assetmap(&entries, dir.path(), crate::Standard::Interop),
            0
        );
        assert!(dir.path().join("ASSETMAP").exists());
        assert!(dir.path().join("VOLINDEX").exists());
        let xml = std::fs::read_to_string(dir.path().join("ASSETMAP")).unwrap();
        assert!(xml.contains("<Id>urn:uuid:pkl-id</Id>"));
        assert!(xml.contains("<PackingList>true</PackingList>"));
        assert!(xml.contains("PROTO-ASDCP-AM-20040311"));
    }
}
