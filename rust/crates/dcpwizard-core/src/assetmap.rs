use serde::{Deserialize, Serialize};
use std::path::Path;

/// ASSETMAP entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetMapEntry {
    pub id: String,
    pub path: String,
}

/// Generate an ASSETMAP and VOLINDEX for the DCP.
pub fn generate_assetmap(entries: &[AssetMapEntry], output_dir: &Path) -> i32 {
    let am_uuid = uuid::Uuid::new_v4();

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<AssetMap xmlns=\"http://www.smpte-ra.org/schemas/429-9/2007/AM\">\n");
    xml.push_str(&format!("  <Id>urn:uuid:{am_uuid}</Id>\n"));
    xml.push_str("  <Creator>DCP Wizard</Creator>\n");
    xml.push_str("  <VolumeCount>1</VolumeCount>\n");
    xml.push_str("  <AssetList>\n");

    for entry in entries {
        xml.push_str("    <Asset>\n");
        xml.push_str(&format!("      <Id>urn:uuid:{}</Id>\n", entry.id));
        xml.push_str("      <ChunkList>\n");
        xml.push_str("        <Chunk>\n");
        xml.push_str(&format!("          <Path>{}</Path>\n", entry.path));
        xml.push_str("        </Chunk>\n");
        xml.push_str("      </ChunkList>\n");
        xml.push_str("    </Asset>\n");
    }

    xml.push_str("  </AssetList>\n");
    xml.push_str("</AssetMap>\n");

    let am_path = output_dir.join("ASSETMAP.xml");
    if let Err(e) = std::fs::write(&am_path, xml) {
        tracing::error!("Failed to write ASSETMAP: {e}");
        return -1;
    }

    // Generate VOLINDEX
    let volindex = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <VolumeIndex xmlns=\"http://www.smpte-ra.org/schemas/429-9/2007/AM\">\n\
        \x20\x20<Index>1</Index>\n\
        </VolumeIndex>\n";
    let vi_path = output_dir.join("VOLINDEX.xml");
    if let Err(e) = std::fs::write(&vi_path, volindex) {
        tracing::error!("Failed to write VOLINDEX: {e}");
        return -1;
    }

    0
}
