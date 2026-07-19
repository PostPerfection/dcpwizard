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
    pub picture_edit_rate_num: u32,
    pub picture_edit_rate_den: u32,
    pub picture_duration: u64,
    pub picture_entry_point: u64,
    pub sound_id: Option<String>,
    pub sound_edit_rate_num: u32,
    pub sound_edit_rate_den: u32,
    pub sound_duration: u64,
    pub sound_entry_point: u64,
}

/// Generate a Composition Playlist XML.
pub fn generate_cpl(config: &CplConfig, cpl_uuid: &str, output_file: &Path) -> i32 {
    let issue_date = time_now_iso();
    let namespace = match config.standard {
        crate::Standard::Smpte => "http://www.smpte-ra.org/schemas/429-7/2006/CPL",
        crate::Standard::Interop => "http://www.digicine.com/PROTO-ASDCP-CPL-20040511#",
    };

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!("<CompositionPlaylist xmlns=\"{namespace}\">\n"));
    xml.push_str(&format!("  <Id>urn:uuid:{cpl_uuid}</Id>\n"));
    xml.push_str(&format!(
        "  <ContentTitleText>{}</ContentTitleText>\n",
        escape_xml(&config.title)
    ));
    xml.push_str(&format!("  <IssueDate>{issue_date}</IssueDate>\n"));
    xml.push_str("  <Issuer>DCP Wizard</Issuer>\n");
    xml.push_str("  <Creator>DCP Wizard</Creator>\n");

    if !config.content_kind.is_empty() {
        xml.push_str(&format!(
            "  <ContentKind>{}</ContentKind>\n",
            escape_xml(&config.content_kind)
        ));
    }

    xml.push_str("  <ReelList>\n");
    for (i, reel) in config.reels.iter().enumerate() {
        xml.push_str("    <Reel>\n");
        xml.push_str(&format!("      <Id>urn:uuid:{}</Id>\n", reel.reel_id));
        xml.push_str(&format!(
            "      <AnnotationText>Reel {}</AnnotationText>\n",
            i + 1
        ));
        xml.push_str("      <AssetList>\n");
        // MainPicture
        xml.push_str("        <MainPicture>\n");
        xml.push_str(&format!(
            "          <Id>urn:uuid:{}</Id>\n",
            reel.picture_id
        ));
        xml.push_str(&format!(
            "          <EditRate>{} {}</EditRate>\n",
            reel.picture_edit_rate_num, reel.picture_edit_rate_den
        ));
        xml.push_str(&format!(
            "          <IntrinsicDuration>{}</IntrinsicDuration>\n",
            reel.picture_duration
        ));
        xml.push_str(&format!(
            "          <EntryPoint>{}</EntryPoint>\n",
            reel.picture_entry_point
        ));
        xml.push_str(&format!(
            "          <Duration>{}</Duration>\n",
            reel.picture_duration
        ));
        xml.push_str(&format!(
            "          <FrameRate>{} {}</FrameRate>\n",
            reel.picture_edit_rate_num, reel.picture_edit_rate_den
        ));
        xml.push_str("          <ScreenAspectRatio>1998 1080</ScreenAspectRatio>\n");
        xml.push_str("        </MainPicture>\n");
        // MainSound (optional)
        if let Some(ref sound_id) = reel.sound_id {
            xml.push_str("        <MainSound>\n");
            xml.push_str(&format!("          <Id>urn:uuid:{sound_id}</Id>\n"));
            xml.push_str(&format!(
                "          <EditRate>{} {}</EditRate>\n",
                reel.sound_edit_rate_num, reel.sound_edit_rate_den
            ));
            xml.push_str(&format!(
                "          <IntrinsicDuration>{}</IntrinsicDuration>\n",
                reel.sound_duration
            ));
            xml.push_str(&format!(
                "          <EntryPoint>{}</EntryPoint>\n",
                reel.sound_entry_point
            ));
            xml.push_str(&format!(
                "          <Duration>{}</Duration>\n",
                reel.sound_duration
            ));
            xml.push_str("        </MainSound>\n");
        }
        xml.push_str("      </AssetList>\n");
        xml.push_str("    </Reel>\n");
    }
    xml.push_str("  </ReelList>\n");
    xml.push_str("</CompositionPlaylist>\n");

    match std::fs::write(output_file, xml) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("Failed to write CPL: {e}");
            -1
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
