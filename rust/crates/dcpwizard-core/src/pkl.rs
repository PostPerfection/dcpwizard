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

/// Generate a Packing List XML.
pub fn generate_pkl(entries: &[PklEntry], output_file: &Path) -> i32 {
    let pkl_uuid = uuid::Uuid::new_v4();

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<PackingList xmlns=\"http://www.smpte-ra.org/schemas/429-8/2007/PKL\">\n");
    xml.push_str(&format!("  <Id>urn:uuid:{pkl_uuid}</Id>\n"));
    xml.push_str("  <Issuer>DCP Wizard</Issuer>\n");
    xml.push_str("  <Creator>DCP Wizard</Creator>\n");
    xml.push_str("  <AssetList>\n");

    for entry in entries {
        xml.push_str("    <Asset>\n");
        xml.push_str(&format!("      <Id>urn:uuid:{}</Id>\n", entry.id));
        xml.push_str(&format!("      <Hash>{}</Hash>\n", entry.hash));
        xml.push_str(&format!("      <Size>{}</Size>\n", entry.size));
        xml.push_str(&format!("      <Type>{}</Type>\n", entry.asset_type));
        xml.push_str("    </Asset>\n");
    }

    xml.push_str("  </AssetList>\n");
    xml.push_str("</PackingList>\n");

    match std::fs::write(output_file, xml) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("Failed to write PKL: {e}");
            -1
        }
    }
}
