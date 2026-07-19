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
    let am_uuid = uuid::Uuid::new_v4();
    let (namespace, assetmap_name, volindex_name) = match standard {
        crate::Standard::Smpte => (
            "http://www.smpte-ra.org/schemas/429-9/2007/AM",
            "ASSETMAP.xml",
            "VOLINDEX.xml",
        ),
        crate::Standard::Interop => (
            "http://www.digicine.com/PROTO-ASDCP-AM-20040311#",
            "ASSETMAP",
            "VOLINDEX",
        ),
    };

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!("<AssetMap xmlns=\"{namespace}\">\n"));
    xml.push_str(&format!("  <Id>urn:uuid:{am_uuid}</Id>\n"));
    xml.push_str("  <Creator>DCP Wizard</Creator>\n");
    xml.push_str("  <VolumeCount>1</VolumeCount>\n");
    xml.push_str("  <AssetList>\n");

    for entry in entries {
        xml.push_str("    <Asset>\n");
        xml.push_str(&format!("      <Id>urn:uuid:{}</Id>\n", entry.id));
        if entry.packing_list {
            xml.push_str("      <PackingList>true</PackingList>\n");
        }
        xml.push_str("      <ChunkList>\n");
        xml.push_str("        <Chunk>\n");
        xml.push_str(&format!("          <Path>{}</Path>\n", entry.path));
        xml.push_str("        </Chunk>\n");
        xml.push_str("      </ChunkList>\n");
        xml.push_str("    </Asset>\n");
    }

    xml.push_str("  </AssetList>\n");
    xml.push_str("</AssetMap>\n");

    let am_path = output_dir.join(assetmap_name);
    if let Err(e) = std::fs::write(&am_path, xml) {
        tracing::error!("Failed to write ASSETMAP: {e}");
        return -1;
    }

    // Generate VOLINDEX
    let volindex = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VolumeIndex xmlns=\"{namespace}\">\n  <Index>1</Index>\n</VolumeIndex>\n"
    );
    let vi_path = output_dir.join(volindex_name);
    if let Err(e) = std::fs::write(&vi_path, volindex) {
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
