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
pub fn generate_pkl(
    entries: &[PklEntry],
    pkl_uuid: &str,
    standard: crate::Standard,
    output_file: &Path,
) -> i32 {
    let namespace = match standard {
        crate::Standard::Smpte => "http://www.smpte-ra.org/schemas/429-8/2007/PKL",
        crate::Standard::Interop => "http://www.digicine.com/PROTO-ASDCP-PKL-20040311#",
    };

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!("<PackingList xmlns=\"{namespace}\">\n"));
    xml.push_str(&format!("  <Id>urn:uuid:{pkl_uuid}</Id>\n"));
    xml.push_str(&format!(
        "  <IssueDate>{}</IssueDate>\n",
        chrono::Utc::now().to_rfc3339()
    ));
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
