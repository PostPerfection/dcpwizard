use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for multi-CPL DCP operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiCplConfig {
    pub dcp_dir: PathBuf,
    pub output_dir: PathBuf,
    pub selected_cpls: Vec<String>,
}

/// A CPL entry found in a DCP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CplEntry {
    pub id: String,
    pub file_path: String,
    pub content_title: String,
    pub content_kind: String,
}

/// Timeline entry representing a reel within a CPL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub reel_id: String,
    pub reel_number: u32,
    pub duration_frames: u64,
    pub entry_point: u64,
    pub edit_rate: String,
    pub picture_asset_id: String,
    pub sound_asset_id: String,
    pub subtitle_asset_id: String,
    pub picture_file: String,
    pub sound_file: String,
    pub subtitle_file: String,
    pub subtitle_language: String,
}

/// List all CPLs in a DCP by parsing the ASSETMAP.
pub fn list_cpls(dcp_dir: &Path) -> Vec<CplEntry> {
    let assetmap_path = find_assetmap(dcp_dir);
    let assetmap_path = match assetmap_path {
        Some(p) => p,
        None => {
            tracing::error!("No ASSETMAP found in {}", dcp_dir.display());
            return Vec::new();
        }
    };

    let content = match std::fs::read_to_string(&assetmap_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read ASSETMAP: {e}");
            return Vec::new();
        }
    };

    let mut cpls = Vec::new();

    // Parse ASSETMAP XML to find CPL assets
    let mut in_asset = false;
    let mut current_id = String::new();
    let mut current_path = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("<Asset>") || trimmed.starts_with("<Asset ") {
            in_asset = true;
            current_id.clear();
            current_path.clear();
        } else if trimmed == "</Asset>" {
            if in_asset && !current_id.is_empty() && !current_path.is_empty() {
                // Check if this is a CPL by reading the file
                let full_path = dcp_dir.join(&current_path);
                if full_path.exists()
                    && let Ok(file_content) = std::fs::read_to_string(&full_path)
                    && file_content.contains("CompositionPlaylist")
                {
                    let title = extract_xml_value(&file_content, "ContentTitleText")
                        .or_else(|| extract_xml_value(&file_content, "ContentTitle"))
                        .unwrap_or_default();
                    let kind = extract_xml_value(&file_content, "ContentKind").unwrap_or_default();

                    cpls.push(CplEntry {
                        id: current_id.clone(),
                        file_path: current_path.clone(),
                        content_title: title,
                        content_kind: kind,
                    });
                }
            }
            in_asset = false;
        } else if in_asset {
            if let Some(id) = extract_xml_value(trimmed, "Id") {
                current_id = id.replace("urn:uuid:", "");
            }
            if let Some(path) = extract_xml_value(trimmed, "Path") {
                current_path = path;
            }
        }
    }

    cpls
}

/// Get the timeline (reel structure) for a specific CPL, resolving asset file paths from the DCP directory.
pub fn get_timeline(cpl_path: &Path) -> Vec<TimelineEntry> {
    let dcp_dir = cpl_path.parent().unwrap_or(Path::new("."));
    let content = match std::fs::read_to_string(cpl_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read CPL: {e}");
            return Vec::new();
        }
    };

    // Build asset UUID → file path map from ASSETMAP
    let asset_map = parse_assetmap(dcp_dir);

    let mut entries = Vec::new();
    let mut reel_number = 0u32;

    // Simple XML parser for reel info
    let mut in_reel = false;
    let mut reel_id = String::new();
    let mut duration = 0u64;
    let mut entry_point = 0u64;
    let mut edit_rate = String::new();
    let mut picture_id = String::new();
    let mut sound_id = String::new();
    let mut subtitle_id = String::new();
    let mut subtitle_lang = String::new();
    let mut in_picture = false;
    let mut in_sound = false;
    let mut in_subtitle = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.contains("<Reel>") || trimmed.contains("<Reel ") {
            in_reel = true;
            reel_number += 1;
            reel_id.clear();
            duration = 0;
            entry_point = 0;
            edit_rate.clear();
            picture_id.clear();
            sound_id.clear();
            subtitle_id.clear();
            subtitle_lang.clear();
            in_picture = false;
            in_sound = false;
            in_subtitle = false;
        } else if trimmed.contains("</Reel>") {
            if in_reel {
                let picture_file = asset_map
                    .get(&picture_id)
                    .map(|p| dcp_dir.join(p).to_string_lossy().into_owned())
                    .unwrap_or_default();
                let sound_file = asset_map
                    .get(&sound_id)
                    .map(|p| dcp_dir.join(p).to_string_lossy().into_owned())
                    .unwrap_or_default();
                let subtitle_file = asset_map
                    .get(&subtitle_id)
                    .map(|p| dcp_dir.join(p).to_string_lossy().into_owned())
                    .unwrap_or_default();
                entries.push(TimelineEntry {
                    reel_id: reel_id.clone(),
                    reel_number,
                    duration_frames: duration,
                    entry_point,
                    edit_rate: edit_rate.clone(),
                    picture_asset_id: picture_id.clone(),
                    sound_asset_id: sound_id.clone(),
                    subtitle_asset_id: subtitle_id.clone(),
                    picture_file,
                    sound_file,
                    subtitle_file,
                    subtitle_language: subtitle_lang.clone(),
                });
            }
            in_reel = false;
        } else if in_reel {
            if trimmed.contains("MainPicture") && trimmed.contains('<') && !trimmed.contains('/') {
                in_picture = true;
                in_sound = false;
                in_subtitle = false;
            } else if trimmed.contains("MainSound")
                && trimmed.contains('<')
                && !trimmed.contains('/')
            {
                in_sound = true;
                in_picture = false;
                in_subtitle = false;
            } else if trimmed.contains("MainSubtitle")
                && trimmed.contains('<')
                && !trimmed.contains('/')
            {
                in_subtitle = true;
                in_picture = false;
                in_sound = false;
            } else if trimmed.contains("</MainPicture>") {
                in_picture = false;
            } else if trimmed.contains("</MainSound>") {
                in_sound = false;
            } else if trimmed.contains("</MainSubtitle>") {
                in_subtitle = false;
            }

            if let Some(id) = extract_xml_value(trimmed, "Id") {
                let clean_id = id.replace("urn:uuid:", "");
                if reel_id.is_empty() && !in_picture && !in_sound && !in_subtitle {
                    reel_id = clean_id;
                } else if in_picture && picture_id.is_empty() {
                    picture_id = clean_id;
                } else if in_sound && sound_id.is_empty() {
                    sound_id = clean_id;
                } else if in_subtitle && subtitle_id.is_empty() {
                    subtitle_id = clean_id;
                }
            }
            if in_subtitle && let Some(lang) = extract_xml_value(trimmed, "Language") {
                subtitle_lang = lang;
            }
            if let Some(d) = extract_xml_value(trimmed, "Duration")
                && let Ok(v) = d.parse::<u64>()
            {
                duration = v;
            }
            if let Some(ep) = extract_xml_value(trimmed, "EntryPoint")
                && let Ok(v) = ep.parse::<u64>()
                && in_picture
            {
                entry_point = v;
            }
            if let Some(er) = extract_xml_value(trimmed, "EditRate")
                && edit_rate.is_empty()
            {
                edit_rate = er;
            }
        }
    }

    entries
}

/// Parse the ASSETMAP to build a UUID → relative file path map.
fn parse_assetmap(dcp_dir: &Path) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let assetmap_path = match find_assetmap(dcp_dir) {
        Some(p) => p,
        None => return map,
    };
    let content = match std::fs::read_to_string(&assetmap_path) {
        Ok(c) => c,
        Err(_) => return map,
    };

    let mut in_asset = false;
    let mut current_id = String::new();
    let mut current_path = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<Asset>") || trimmed.starts_with("<Asset ") {
            in_asset = true;
            current_id.clear();
            current_path.clear();
        } else if trimmed == "</Asset>" {
            if in_asset && !current_id.is_empty() && !current_path.is_empty() {
                map.insert(current_id.clone(), current_path.clone());
            }
            in_asset = false;
        } else if in_asset {
            if let Some(id) = extract_xml_value(trimmed, "Id") {
                current_id = id.replace("urn:uuid:", "");
            }
            if let Some(path) = extract_xml_value(trimmed, "Path") {
                current_path = path;
            }
        }
    }

    map
}

/// Create a multi-CPL DCP by copying selected CPLs and their referenced assets.
pub fn create_multi_cpl(config: &MultiCplConfig) -> i32 {
    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    let all_cpls = list_cpls(&config.dcp_dir);
    let selected: Vec<&CplEntry> = if config.selected_cpls.is_empty() {
        all_cpls.iter().collect()
    } else {
        all_cpls
            .iter()
            .filter(|c| config.selected_cpls.contains(&c.id))
            .collect()
    };

    if selected.is_empty() {
        tracing::error!("No matching CPLs found");
        return -1;
    }

    // Copy each selected CPL and referenced assets
    for cpl in &selected {
        let src = config.dcp_dir.join(&cpl.file_path);
        let dst = config.output_dir.join(&cpl.file_path);
        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::copy(&src, &dst) {
            tracing::error!("Failed to copy CPL {}: {e}", cpl.id);
            return -1;
        }
    }

    // Copy ASSETMAP and VOLINDEX
    for name in &["ASSETMAP", "ASSETMAP.xml", "VOLINDEX", "VOLINDEX.xml"] {
        let src = config.dcp_dir.join(name);
        if src.exists() {
            let _ = std::fs::copy(&src, config.output_dir.join(name));
        }
    }

    // Copy PKL files
    let pkl_files: Vec<PathBuf> = std::fs::read_dir(&config.dcp_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("PKL") && n.ends_with(".xml"))
        })
        .collect();

    for pkl in &pkl_files {
        if let Some(name) = pkl.file_name() {
            let _ = std::fs::copy(pkl, config.output_dir.join(name));
        }
    }

    // Copy MXF files referenced by selected CPLs
    let mxf_files: Vec<PathBuf> = std::fs::read_dir(&config.dcp_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("mxf"))
        })
        .collect();

    for mxf in &mxf_files {
        if let Some(name) = mxf.file_name() {
            let dst = config.output_dir.join(name);
            if !dst.exists()
                && let Err(e) = std::fs::copy(mxf, &dst)
            {
                tracing::error!("Failed to copy MXF {}: {e}", mxf.display());
                return -1;
            }
        }
    }

    tracing::info!(
        "Created multi-CPL DCP with {} CPLs at {}",
        selected.len(),
        config.output_dir.display()
    );
    0
}

fn find_assetmap(dir: &Path) -> Option<PathBuf> {
    for name in &["ASSETMAP", "ASSETMAP.xml"] {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn extract_xml_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start_pos = text.find(&open)?;
    let after_open = &text[start_pos + open.len()..];
    let content_start = after_open.find('>')?;
    let content = &after_open[content_start + 1..];
    let end_pos = content.find(&close)?;
    let value = content[..end_pos].trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}
