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
    /// Bare UUID of the timed-text (subtitle) track, when present.
    pub subtitle_id: Option<String>,
    pub subtitle_edit_rate_num: u32,
    pub subtitle_edit_rate_den: u32,
    pub subtitle_duration: u64,
    pub subtitle_entry_point: u64,
    pub subtitle_language: Option<String>,
    /// Picture is a stereoscopic (ST 429-10) essence: emit MainStereoscopicPicture
    /// with FrameRate doubled (two frames per edit unit) instead of MainPicture.
    pub stereoscopic: bool,
    /// Auxiliary data (ST 429-18) track, e.g. Dolby Atmos. When present, an
    /// AuxData element is added to the reel's AssetList.
    pub aux_data: Option<AuxData>,
}

/// Auxiliary-data (ST 429-18) track for a CPL reel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuxData {
    pub id: String,
    pub edit_rate_num: u32,
    pub edit_rate_den: u32,
    pub duration: u64,
    pub entry_point: u64,
    pub key_id: Option<String>,
    /// SMPTE data-essence UL (e.g. Dolby Atmos IAB bitstream).
    pub data_type: String,
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

    // postkit's DcpCplReel only writes MainPicture/MainSound, so splice the extra
    // per-reel assets (subtitle, aux data) and rewrite the picture element for
    // stereoscopic reels. Reel order is preserved.
    let mut xml = cpl.to_xml();
    let needs_splice = config
        .reels
        .iter()
        .any(|r| r.subtitle_id.is_some() || r.aux_data.is_some() || r.stereoscopic);
    if needs_splice {
        xml = splice_reel_extras(&xml, &config.reels, config.standard);
    }

    match std::fs::write(output_file, xml) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!("Failed to write CPL: {e}");
            -1
        }
    }
}

/// ST 429-10 (2008) stereoscopic picture namespace.
const NS_STEREO_429_10: &str =
    "http://www.smpte-ra.org/schemas/429-10/2008/Main-Stereo-Picture-CPL";
/// Dolby auxiliary-data (ST 429-18) namespace used by real Atmos DCPs.
const NS_AUX_DATA: &str = "http://www.dolby.com/schemas/2012/AD";

/// Rewrite each reel's picture element and add the extra AssetList entries
/// (subtitle, aux data) that postkit's writer does not emit. Walks the reel
/// segments in order between `</AssetList>` markers.
fn splice_reel_extras(xml: &str, reels: &[CplReel], standard: crate::Standard) -> String {
    const MARKER: &str = "      </AssetList>\n";
    let mut out = String::with_capacity(xml.len() + 512);
    let mut idx = 0;
    for reel in reels {
        let Some(pos) = xml[idx..].find(MARKER) else {
            break;
        };
        let end = idx + pos;
        let mut segment = xml[idx..end].to_string();
        if reel.stereoscopic {
            segment = rewrite_stereoscopic(&segment, reel, standard);
        }
        out.push_str(&segment);
        if let Some(ref sid) = reel.subtitle_id {
            out.push_str(&main_subtitle_block(reel, sid));
        }
        if let Some(ref aux) = reel.aux_data {
            out.push_str(&aux_data_block(aux));
        }
        out.push_str(MARKER);
        idx = end + MARKER.len();
    }
    out.push_str(&xml[idx..]);
    out
}

/// Replace a reel segment's `<MainPicture>…</MainPicture>` block with a ST 429-10
/// MainStereoscopicPicture (FrameRate = 2x edit rate for interleaved L/R).
fn rewrite_stereoscopic(segment: &str, reel: &CplReel, standard: crate::Standard) -> String {
    const OPEN: &str = "        <MainPicture>\n";
    const CLOSE: &str = "        </MainPicture>\n";
    let (Some(start), Some(close)) = (segment.find(OPEN), segment.find(CLOSE)) else {
        return segment.to_string();
    };
    let block_end = close + CLOSE.len();
    let mut out = String::with_capacity(segment.len() + 128);
    out.push_str(&segment[..start]);
    out.push_str(&stereoscopic_picture_block(reel, standard));
    out.push_str(&segment[block_end..]);
    out
}

fn stereoscopic_picture_block(reel: &CplReel, standard: crate::Standard) -> String {
    // prefixed form: the element sits in the 429-10 namespace while its children
    // (Id, EditRate, ...) inherit the CPL's 429-7 default namespace. This is the
    // form that validates against the 429-10 XSD (whose base type is 429-7's
    // PictureTrackFileAssetType) and matches real production 3D DCPs; the
    // default-xmlns form used by some samples fails strict schema validation.
    let mut b = String::new();
    b.push_str(&format!(
        "        <msp-cpl:MainStereoscopicPicture xmlns:msp-cpl=\"{NS_STEREO_429_10}\">\n"
    ));
    b.push_str(&format!(
        "          <Id>urn:uuid:{}</Id>\n",
        reel.picture_id
    ));
    b.push_str(&format!(
        "          <EditRate>{} {}</EditRate>\n",
        reel.picture_edit_rate_num, reel.picture_edit_rate_den
    ));
    b.push_str(&format!(
        "          <IntrinsicDuration>{}</IntrinsicDuration>\n",
        reel.picture_duration
    ));
    b.push_str(&format!(
        "          <EntryPoint>{}</EntryPoint>\n",
        reel.picture_entry_point
    ));
    b.push_str(&format!(
        "          <Duration>{}</Duration>\n",
        reel.picture_duration
    ));
    if let Some(ref key_id) = reel.picture_key_id {
        b.push_str(&format!("          <KeyId>urn:uuid:{key_id}</KeyId>\n"));
    }
    // stereoscopic essence carries two frames (left+right) per edit unit
    b.push_str(&format!(
        "          <FrameRate>{} {}</FrameRate>\n",
        reel.picture_edit_rate_num * 2,
        reel.picture_edit_rate_den
    ));
    match standard {
        crate::Standard::Interop => b.push_str(&format!(
            "          <ScreenAspectRatio>{}</ScreenAspectRatio>\n",
            interop_aspect(reel.picture_width, reel.picture_height)
        )),
        crate::Standard::Smpte => b.push_str(&format!(
            "          <ScreenAspectRatio>{} {}</ScreenAspectRatio>\n",
            reel.picture_width, reel.picture_height
        )),
    }
    b.push_str("        </msp-cpl:MainStereoscopicPicture>\n");
    b
}

/// Interop ScreenAspectRatio is a decimal ratio rounded to 2 dp.
fn interop_aspect(w: u32, h: u32) -> String {
    if h == 0 {
        return "0".to_string();
    }
    format!("{:.2}", w as f64 / h as f64)
}

fn aux_data_block(aux: &AuxData) -> String {
    let mut b = String::new();
    b.push_str(&format!(
        "        <axd:AuxData xmlns:axd=\"{NS_AUX_DATA}\">\n"
    ));
    b.push_str(&format!("          <Id>urn:uuid:{}</Id>\n", aux.id));
    b.push_str(&format!(
        "          <EditRate>{} {}</EditRate>\n",
        aux.edit_rate_num, aux.edit_rate_den
    ));
    b.push_str(&format!(
        "          <IntrinsicDuration>{}</IntrinsicDuration>\n",
        aux.duration
    ));
    b.push_str(&format!(
        "          <EntryPoint>{}</EntryPoint>\n",
        aux.entry_point
    ));
    b.push_str(&format!(
        "          <Duration>{}</Duration>\n",
        aux.duration
    ));
    if let Some(ref key_id) = aux.key_id {
        b.push_str(&format!("          <KeyId>urn:uuid:{key_id}</KeyId>\n"));
    }
    b.push_str(&format!(
        "          <axd:DataType>{}</axd:DataType>\n",
        aux.data_type
    ));
    b.push_str("        </axd:AuxData>\n");
    b
}

fn main_subtitle_block(reel: &CplReel, subtitle_id: &str) -> String {
    let mut b = String::new();
    b.push_str("        <MainSubtitle>\n");
    b.push_str(&format!("          <Id>urn:uuid:{subtitle_id}</Id>\n"));
    b.push_str(&format!(
        "          <EditRate>{} {}</EditRate>\n",
        reel.subtitle_edit_rate_num, reel.subtitle_edit_rate_den
    ));
    b.push_str(&format!(
        "          <IntrinsicDuration>{}</IntrinsicDuration>\n",
        reel.subtitle_duration
    ));
    b.push_str(&format!(
        "          <EntryPoint>{}</EntryPoint>\n",
        reel.subtitle_entry_point
    ));
    b.push_str(&format!(
        "          <Duration>{}</Duration>\n",
        reel.subtitle_duration
    ));
    if let Some(ref lang) = reel.subtitle_language {
        b.push_str(&format!("          <Language>{lang}</Language>\n"));
    }
    b.push_str("        </MainSubtitle>\n");
    b
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

    fn stereo_reel() -> CplReel {
        CplReel {
            reel_id: "r1".into(),
            picture_id: "pic1".into(),
            picture_width: 2048,
            picture_height: 858,
            picture_edit_rate_num: 24,
            picture_edit_rate_den: 1,
            picture_duration: 240,
            stereoscopic: true,
            ..Default::default()
        }
    }

    #[test]
    fn stereoscopic_reel_emits_main_stereoscopic_picture() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CPL.xml");
        let config = CplConfig {
            title: "3D".into(),
            content_kind: "feature".into(),
            reels: vec![stereo_reel()],
            ..Default::default()
        };
        assert_eq!(generate_cpl(&config, "cpl1", &path), 0);
        let xml = std::fs::read_to_string(&path).unwrap();
        assert!(!xml.contains("<MainPicture>"), "picture replaced by stereo");
        assert!(xml.contains(
            "<msp-cpl:MainStereoscopicPicture xmlns:msp-cpl=\"http://www.smpte-ra.org/schemas/429-10/2008/Main-Stereo-Picture-CPL\">"
        ));
        // edit rate is the composition rate; frame rate is doubled (L+R interleaved)
        assert!(xml.contains("<EditRate>24 1</EditRate>"));
        assert!(xml.contains("<FrameRate>48 1</FrameRate>"));
        assert!(xml.contains("</msp-cpl:MainStereoscopicPicture>"));
    }

    #[test]
    fn aux_data_reel_emits_axd_auxdata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CPL.xml");
        let mut reel = CplReel {
            reel_id: "r1".into(),
            picture_id: "pic1".into(),
            picture_width: 2048,
            picture_height: 858,
            picture_edit_rate_num: 24,
            picture_edit_rate_den: 1,
            picture_duration: 240,
            ..Default::default()
        };
        reel.aux_data = Some(AuxData {
            id: "aux1".into(),
            edit_rate_num: 24,
            edit_rate_den: 1,
            duration: 240,
            entry_point: 0,
            key_id: None,
            data_type: "urn:smpte:ul:060e2b34.04010105.0e090604.00000000".into(),
        });
        let config = CplConfig {
            title: "Atmos".into(),
            content_kind: "feature".into(),
            reels: vec![reel],
            ..Default::default()
        };
        assert_eq!(generate_cpl(&config, "cpl1", &path), 0);
        let xml = std::fs::read_to_string(&path).unwrap();
        assert!(xml.contains("<axd:AuxData xmlns:axd=\"http://www.dolby.com/schemas/2012/AD\">"));
        assert!(xml.contains("<Id>urn:uuid:aux1</Id>"));
        assert!(xml.contains(
            "<axd:DataType>urn:smpte:ul:060e2b34.04010105.0e090604.00000000</axd:DataType>"
        ));
        // aux data sits after MainPicture/MainSound, before </AssetList>
        let aux_pos = xml.find("<axd:AuxData").unwrap();
        let close_pos = xml.find("</AssetList>").unwrap();
        assert!(aux_pos < close_pos);
    }
}
