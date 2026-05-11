use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Version File DCP configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VfConfig {
    pub ov_dir: PathBuf,
    pub vf_dir: PathBuf,
    pub title: String,
    pub replacement_reels: Vec<ReplacementReel>,
}

/// A reel in the VF that replaces or references an OV reel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplacementReel {
    pub reel_number: u32,
    pub picture_mxf: Option<PathBuf>,
    pub sound_mxf: Option<PathBuf>,
}

/// Create a Version File DCP that references the Original Version.
///
/// The VF CPL references assets from the OV where no replacement is provided,
/// and uses new assets for replaced reels.
pub fn create_vf(config: &VfConfig) -> i32 {
    if !config.ov_dir.exists() {
        tracing::error!("OV directory not found: {}", config.ov_dir.display());
        return -1;
    }

    if let Err(e) = std::fs::create_dir_all(&config.vf_dir) {
        tracing::error!("Failed to create VF directory: {e}");
        return -1;
    }

    // Read the OV CPL to get reel structure
    let ov_cpls = crate::multi_cpl::list_cpls(&config.ov_dir);
    let ov_cpl = match ov_cpls.first() {
        Some(c) => c,
        None => {
            tracing::error!("No CPL found in OV directory");
            return -1;
        }
    };

    let ov_cpl_path = config.ov_dir.join(&ov_cpl.file_path);
    let ov_timeline = crate::multi_cpl::get_timeline(&ov_cpl_path);

    let vf_cpl_uuid = uuid::Uuid::new_v4();
    let title = if config.title.is_empty() {
        format!("{}_VF", ov_cpl.content_title)
    } else {
        config.title.clone()
    };

    // Build VF CPL XML
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(
        "<CompositionPlaylist xmlns=\"http://www.smpte-ra.org/schemas/429-7/2006/CPL\">\n",
    );
    xml.push_str(&format!("  <Id>urn:uuid:{vf_cpl_uuid}</Id>\n"));
    xml.push_str(&format!(
        "  <ContentTitleText>{}</ContentTitleText>\n",
        escape_xml(&title)
    ));
    xml.push_str("  <IssueDate>2024-01-01T00:00:00+00:00</IssueDate>\n");
    xml.push_str("  <Issuer>DCP Wizard</Issuer>\n");
    xml.push_str("  <Creator>DCP Wizard</Creator>\n");
    xml.push_str("  <ContentKind>feature</ContentKind>\n");
    xml.push_str("  <ReelList>\n");

    for entry in &ov_timeline {
        let replacement = config
            .replacement_reels
            .iter()
            .find(|r| r.reel_number == entry.reel_number);

        xml.push_str("    <Reel>\n");
        xml.push_str(&format!(
            "      <Id>urn:uuid:{}</Id>\n",
            uuid::Uuid::new_v4()
        ));
        xml.push_str("      <AssetList>\n");

        // Picture
        if let Some(rep) = replacement {
            if let Some(pic) = &rep.picture_mxf {
                // Copy new picture MXF to VF directory
                if let Some(name) = pic.file_name() {
                    let _ = std::fs::copy(pic, config.vf_dir.join(name));
                }
                let pic_uuid = uuid::Uuid::new_v4();
                xml.push_str("        <MainPicture>\n");
                xml.push_str(&format!("          <Id>urn:uuid:{pic_uuid}</Id>\n"));
                xml.push_str(&format!(
                    "          <Duration>{}</Duration>\n",
                    entry.duration_frames
                ));
                xml.push_str(&format!(
                    "          <EditRate>{}</EditRate>\n",
                    entry.edit_rate
                ));
                xml.push_str("        </MainPicture>\n");
            } else {
                // Reference OV picture
                xml.push_str("        <MainPicture>\n");
                xml.push_str(&format!(
                    "          <Id>urn:uuid:{}</Id>\n",
                    entry.picture_asset_id
                ));
                xml.push_str(&format!(
                    "          <Duration>{}</Duration>\n",
                    entry.duration_frames
                ));
                xml.push_str(&format!(
                    "          <EditRate>{}</EditRate>\n",
                    entry.edit_rate
                ));
                xml.push_str("        </MainPicture>\n");
            }

            if let Some(snd) = &rep.sound_mxf {
                if let Some(name) = snd.file_name() {
                    let _ = std::fs::copy(snd, config.vf_dir.join(name));
                }
                let snd_uuid = uuid::Uuid::new_v4();
                xml.push_str("        <MainSound>\n");
                xml.push_str(&format!("          <Id>urn:uuid:{snd_uuid}</Id>\n"));
                xml.push_str(&format!(
                    "          <Duration>{}</Duration>\n",
                    entry.duration_frames
                ));
                xml.push_str(&format!(
                    "          <EditRate>{}</EditRate>\n",
                    entry.edit_rate
                ));
                xml.push_str("        </MainSound>\n");
            } else if !entry.sound_asset_id.is_empty() {
                xml.push_str("        <MainSound>\n");
                xml.push_str(&format!(
                    "          <Id>urn:uuid:{}</Id>\n",
                    entry.sound_asset_id
                ));
                xml.push_str(&format!(
                    "          <Duration>{}</Duration>\n",
                    entry.duration_frames
                ));
                xml.push_str(&format!(
                    "          <EditRate>{}</EditRate>\n",
                    entry.edit_rate
                ));
                xml.push_str("        </MainSound>\n");
            }
        } else {
            // Reference OV assets directly
            xml.push_str("        <MainPicture>\n");
            xml.push_str(&format!(
                "          <Id>urn:uuid:{}</Id>\n",
                entry.picture_asset_id
            ));
            xml.push_str(&format!(
                "          <Duration>{}</Duration>\n",
                entry.duration_frames
            ));
            xml.push_str(&format!(
                "          <EditRate>{}</EditRate>\n",
                entry.edit_rate
            ));
            xml.push_str("        </MainPicture>\n");

            if !entry.sound_asset_id.is_empty() {
                xml.push_str("        <MainSound>\n");
                xml.push_str(&format!(
                    "          <Id>urn:uuid:{}</Id>\n",
                    entry.sound_asset_id
                ));
                xml.push_str(&format!(
                    "          <Duration>{}</Duration>\n",
                    entry.duration_frames
                ));
                xml.push_str(&format!(
                    "          <EditRate>{}</EditRate>\n",
                    entry.edit_rate
                ));
                xml.push_str("        </MainSound>\n");
            }
        }

        xml.push_str("      </AssetList>\n");
        xml.push_str("    </Reel>\n");
    }

    xml.push_str("  </ReelList>\n");
    xml.push_str("</CompositionPlaylist>\n");

    let cpl_path = config.vf_dir.join(format!("CPL_{vf_cpl_uuid}.xml"));
    if let Err(e) = std::fs::write(&cpl_path, &xml) {
        tracing::error!("Failed to write VF CPL: {e}");
        return -1;
    }

    // Generate PKL for the VF
    let pkl_uuid = uuid::Uuid::new_v4();
    let cpl_hash = crate::hash::hash_file(&cpl_path).unwrap_or_default();
    let cpl_size = std::fs::metadata(&cpl_path).map(|m| m.len()).unwrap_or(0);

    let pkl_entries = vec![crate::pkl::PklEntry {
        id: vf_cpl_uuid.to_string(),
        asset_type: "text/xml".into(),
        file: cpl_path.clone(),
        hash: cpl_hash,
        size: cpl_size,
    }];

    let pkl_path = config.vf_dir.join(format!("PKL_{pkl_uuid}.xml"));
    if crate::pkl::generate_pkl(&pkl_entries, &pkl_path) != 0 {
        tracing::error!("Failed to generate VF PKL");
        return -1;
    }

    // Generate ASSETMAP
    let am_entries = vec![crate::assetmap::AssetMapEntry {
        id: vf_cpl_uuid.to_string(),
        path: cpl_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
    }];
    if crate::assetmap::generate_assetmap(&am_entries, &config.vf_dir) != 0 {
        tracing::error!("Failed to generate VF ASSETMAP");
        return -1;
    }

    tracing::info!("Created VF DCP at {}", config.vf_dir.display());
    0
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
