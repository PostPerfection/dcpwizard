use serde::{Deserialize, Serialize};
use std::path::Path;

/// CPL configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CplConfig {
    pub title: String,
    pub content_kind: String,
    pub rating: String,
    pub reels: Vec<CplReel>,
    pub standard: crate::Standard,
}

/// A single reel in the CPL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CplReel {
    pub reel_id: String,
    pub picture_id: String,
    pub picture_width: u32,
    pub picture_height: u32,
    pub picture_edit_rate_num: u32,
    pub picture_edit_rate_den: u32,
    pub picture_duration: u64,
    pub picture_entry_point: u64,
    /// KeyId (bare UUID) when the picture essence is encrypted.
    pub picture_key_id: Option<String>,
    pub sound_id: Option<String>,
    pub sound_edit_rate_num: u32,
    pub sound_edit_rate_den: u32,
    pub sound_duration: u64,
    pub sound_entry_point: u64,
    /// KeyId (bare UUID) when the sound essence is encrypted.
    pub sound_key_id: Option<String>,
}

/// Generate a Composition Playlist XML via the shared postkit writer.
pub fn generate_cpl(config: &CplConfig, cpl_uuid: &str, output_file: &Path) -> i32 {
    use postkit::packaging::{self, DcpCpl, DcpCplReel};

    let namespace = match config.standard {
        crate::Standard::Smpte => packaging::ns::CPL_SMPTE,
        crate::Standard::Interop => packaging::ns::CPL_INTEROP,
    };

    let reels = config
        .reels
        .iter()
        .map(|r| DcpCplReel {
            reel_id: r.reel_id.clone(),
            picture_id: r.picture_id.clone(),
            picture_width: r.picture_width,
            picture_height: r.picture_height,
            picture_edit_rate_num: r.picture_edit_rate_num,
            picture_edit_rate_den: r.picture_edit_rate_den,
            picture_duration: r.picture_duration,
            picture_entry_point: r.picture_entry_point,
            picture_key_id: r.picture_key_id.clone(),
            sound_id: r.sound_id.clone(),
            sound_edit_rate_num: r.sound_edit_rate_num,
            sound_edit_rate_den: r.sound_edit_rate_den,
            sound_duration: r.sound_duration,
            sound_entry_point: r.sound_entry_point,
            sound_key_id: r.sound_key_id.clone(),
        })
        .collect();

    let cpl = DcpCpl {
        uuid: cpl_uuid.to_string(),
        namespace: namespace.to_string(),
        title: config.title.clone(),
        content_kind: config.content_kind.clone(),
        issuer: "DCP Wizard".into(),
        creator: "DCP Wizard".into(),
        issue_date: time_now_iso(),
        reels,
    };

    match std::fs::write(output_file, cpl.to_xml()) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("Failed to write CPL: {e}");
            -1
        }
    }
}

fn time_now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_cpl_uses_supplied_identity_and_standard() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CPL_expected.xml");
        let config = CplConfig {
            title: "Interop Test".into(),
            content_kind: "test".into(),
            standard: crate::Standard::Interop,
            ..Default::default()
        };

        assert_eq!(generate_cpl(&config, "expected", &path), 0);
        let xml = std::fs::read_to_string(path).unwrap();
        assert!(xml.contains("<Id>urn:uuid:expected</Id>"));
        assert!(xml.contains("<ContentTitleText>Interop Test</ContentTitleText>"));
        assert!(xml.contains("PROTO-ASDCP-CPL-20040511"));
    }
}
