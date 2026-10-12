//! Assemble one new OV composition from one or more existing DCPs (dom#1675).
//!
//! Builds a single new CPL whose reels are the source DCPs' reels in input order
//! (a program, e.g. short + feature), with library items conformed into reels of
//! their own before the first input's reels and after the last's. Source essence
//! is copied byte-identical and referenced by its existing asset UUIDs. Inputs
//! must agree on standard, frame rate and resolution, and none may be encrypted
//! (a new CPL id breaks the KDM's CPL binding).

use crate::dcp::ProgressSink;
use crate::ingest_package::{detect_standard, file_name, is_cpl_path, tag};
use crate::isdcf_name::Rating;
use crate::library::AttachedItem;
use crate::multi_cpl::{TimelineEntry, get_timeline};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const STARTING_PERCENT: u32 = 2;
const COPY_START_PERCENT: u32 = 5;
const COPY_END_PERCENT: u32 = 80;
const ITEMS_PERCENT: u32 = 80;
const PACKAGE_PERCENT: u32 = 95;
const DONE_PERCENT: u32 = 100;

/// How to assemble the output OV.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssembleConfig {
    pub inputs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub title: String,
    #[serde(default)]
    pub signer: Option<crate::package_signature::PackageSigner>,
    #[serde(default)]
    pub head_items: Vec<AttachedItem>,
    #[serde(default)]
    pub tail_items: Vec<AttachedItem>,
    // None carries the first input's RatingList
    #[serde(default)]
    pub ratings: Option<Vec<Rating>>,
    // None encodes the item reels to the compression ratio alone, as create does
    #[serde(default)]
    pub item_video_bit_rate_mbps: Option<u32>,
    // the running process's settings, never read from a job
    #[serde(skip)]
    pub encode_threads: u32,
    #[serde(skip)]
    pub detect_picture_findings: bool,
}

/// One parsed input DCP: its reels, picture geometry, edit rate and the
/// composition identity the new CPL carries from the first input.
struct InputDcp {
    dir: PathBuf,
    standard: crate::Standard,
    reels: Vec<TimelineEntry>,
    geometry: crate::cpl::PictureGeometry,
    edit_rate: String,
    ratings: Vec<Rating>,
    content_version_label: Option<String>,
}

/// A source track file and the name it is copied to in the output.
struct TrackCopy {
    id: String,
    source: PathBuf,
    name: String,
    recorded: Option<(String, u64)>,
}

pub const ASSEMBLED_OV_TITLE: &str = "Assembled OV";

pub fn assemble_title(title: &str) -> String {
    if title.is_empty() {
        ASSEMBLED_OV_TITLE.to_string()
    } else {
        title.to_string()
    }
}

/// Assemble the inputs into a single-CPL OV at `config.output_dir`. Returns 0 ok.
pub fn assemble(config: &AssembleConfig) -> i32 {
    match assemble_with_progress(config, &crate::dcp::NoProgress) {
        Ok(()) => 0,
        Err(message) => {
            tracing::error!("{message}");
            -1
        }
    }
}

/// As [`assemble`], reporting stage progress and honouring cancellation between
/// copied track files.
pub fn assemble_with_progress(
    config: &AssembleConfig,
    progress: &dyn ProgressSink,
) -> Result<(), String> {
    if config.inputs.is_empty() {
        return Err("assemble needs at least one input DCP".into());
    }
    if progress.cancelled() {
        return Err(crate::dcp::CANCELLED_MESSAGE.into());
    }
    progress.stage(STARTING_PERCENT, "reading the inputs");
    // prove the signer works before anything is written, so a bad one cannot
    // leave a half-signed package behind
    if let Some(signer) = config.signer.as_ref() {
        signer
            .check_usable()
            .map_err(|e| format!("unusable signer: {e}"))?;
    }

    let inputs = config
        .inputs
        .iter()
        .map(|dir| parse_input(dir).map_err(|e| format!("{}: {e}", dir.display())))
        .collect::<Result<Vec<_>, _>>()?;
    check_inputs_agree(&inputs)?;
    let first = &inputs[0];
    let standard = first.standard;

    std::fs::create_dir_all(&config.output_dir)
        .map_err(|e| format!("cannot create output {}: {e}", config.output_dir.display()))?;

    let (edit_num, edit_den) = parse_edit_rate(&first.edit_rate);

    let mut feature_reels: Vec<crate::cpl::CplReel> = Vec::new();
    let mut copies: Vec<TrackCopy> = Vec::new();
    let mut planned: HashSet<String> = HashSet::new();
    let mut used_names: HashSet<String> = HashSet::new();
    let mut main_sound_track: Option<PathBuf> = None;

    for input in &inputs {
        let pkl_map = parse_pkls(&input.dir);
        for reel in &input.reels {
            // copy every track file the reel references, deduped by asset id
            for (id, source) in [
                (&reel.picture_asset_id, &reel.picture_file),
                (&reel.sound_asset_id, &reel.sound_file),
                (&reel.subtitle_asset_id, &reel.subtitle_file),
            ] {
                if id.is_empty() || !planned.insert(id.clone()) {
                    continue;
                }
                let source = PathBuf::from(source);
                if !source.is_file() {
                    return Err(format!("asset {id} file missing: {}", source.display()));
                }
                copies.push(TrackCopy {
                    id: id.clone(),
                    name: unique_name(&file_name(&source), id, &mut used_names),
                    source,
                    recorded: pkl_map.get(id).cloned(),
                });
            }
            if main_sound_track.is_none() && !reel.sound_asset_id.is_empty() {
                main_sound_track = Some(PathBuf::from(&reel.sound_file));
            }

            let has_sub = !reel.subtitle_asset_id.is_empty();
            feature_reels.push(crate::cpl::CplReel {
                reel_id: uuid::Uuid::new_v4().to_string(),
                picture_id: reel.picture_asset_id.clone(),
                picture_width: input.geometry.stored_width,
                picture_height: input.geometry.stored_height,
                picture_active_width: input.geometry.active_width,
                picture_active_height: input.geometry.active_height,
                picture_edit_rate_num: edit_num,
                picture_edit_rate_den: edit_den,
                picture_duration: reel.duration_frames,
                picture_entry_point: reel.entry_point,
                picture_key_id: None,
                sound_id: (!reel.sound_asset_id.is_empty()).then(|| reel.sound_asset_id.clone()),
                sound_edit_rate_num: edit_num,
                sound_edit_rate_den: edit_den,
                sound_duration: reel.duration_frames,
                sound_entry_point: reel.sound_entry_point,
                sound_key_id: None,
                subtitle_id: has_sub.then(|| reel.subtitle_asset_id.clone()),
                subtitle_edit_rate_num: if has_sub { edit_num } else { 0 },
                subtitle_edit_rate_den: if has_sub { edit_den } else { 0 },
                subtitle_duration: if has_sub { reel.duration_frames } else { 0 },
                subtitle_entry_point: 0,
                subtitle_language: has_sub
                    .then(|| reel.subtitle_language.clone())
                    .filter(|l| !l.is_empty()),
                stereoscopic: false,
                aux_data: None,
                ..Default::default()
            });
        }
    }

    if feature_reels.is_empty() {
        return Err("no reels found in the input DCPs".into());
    }

    let (mut pkl_entries, mut assetmap_entries) =
        copy_tracks(&copies, &config.output_dir, progress)?;

    let has_items = !config.head_items.is_empty() || !config.tail_items.is_empty();
    let mut joined = if has_items {
        if progress.cancelled() {
            return Err(crate::dcp::CANCELLED_MESSAGE.into());
        }
        progress.stage(ITEMS_PERCENT, "conforming library items");
        let settings = crate::library_reel::ItemEncodeSettings {
            output_dir: config.output_dir.clone(),
            max_bitrate_mbps: config.item_video_bit_rate_mbps.unwrap_or(0),
            encode_threads: config.encode_threads,
            detect_picture_findings: config.detect_picture_findings,
            encrypt: false,
            audio_language: main_sound_track.as_deref().and_then(spoken_language),
        };
        crate::library_reel::join_library_items(
            &settings,
            &config.head_items,
            &config.tail_items,
            &item_format(first, main_sound_track.as_deref())?,
            feature_reels,
        )?
    } else {
        crate::library_reel::JoinedComposition {
            reels: feature_reels,
            ..Default::default()
        }
    };
    pkl_entries.append(&mut joined.pkl);
    assetmap_entries.append(&mut joined.assetmap);
    // the assembled program is a new composition, so it gets its own FFOC/LFOC
    crate::cpl::apply_default_markers(&mut joined.reels);

    if progress.cancelled() {
        return Err(crate::dcp::CANCELLED_MESSAGE.into());
    }
    progress.stage(PACKAGE_PERCENT, "writing CPL/PKL/ASSETMAP");

    // the program combines several sources, so no input CPL's block describes it
    let main_sound = match main_sound_track {
        Some(ref track) if standard == crate::Standard::Smpte => {
            Some(crate::cpl::main_sound_from_track_file(track)?)
        }
        _ => None,
    };

    // ── CPL ──
    let cpl_uuid = uuid::Uuid::new_v4().to_string();
    let cpl_path = config.output_dir.join(format!("CPL_{cpl_uuid}.xml"));
    let cpl_config = crate::cpl::CplConfig {
        title: assemble_title(&config.title),
        content_kind: "feature".into(),
        ratings: config
            .ratings
            .clone()
            .unwrap_or_else(|| first.ratings.clone()),
        content_version_label: first.content_version_label.clone(),
        reels: joined.reels,
        standard,
        main_sound,
        ..Default::default()
    };
    if crate::cpl::generate_cpl(&cpl_config, &cpl_uuid, &cpl_path) != 0 {
        return Err("failed to write assembled CPL".into());
    }
    // before the PKL entry below hashes it
    if !crate::package_signature::sign_if_configured(config.signer.as_ref(), &cpl_path, "CPL") {
        return Err("signing the CPL failed".into());
    }

    // ── PKL: CPL plus every copied and conformed asset ──
    let pkl_uuid = uuid::Uuid::new_v4().to_string();
    pkl_entries.insert(
        0,
        crate::pkl::PklEntry {
            id: cpl_uuid.clone(),
            asset_type: "text/xml".into(),
            file: cpl_path.clone(),
            hash: crate::hash::hash_file(&cpl_path)?,
            size: file_size(&cpl_path)?,
        },
    );
    let pkl_path = config.output_dir.join(format!("PKL_{pkl_uuid}.xml"));
    if crate::pkl::generate_pkl(&pkl_entries, &pkl_uuid, standard, None, &pkl_path) != 0 {
        return Err("failed to write assembled PKL".into());
    }
    if !crate::package_signature::sign_if_configured(config.signer.as_ref(), &pkl_path, "PKL") {
        return Err("signing the PKL failed".into());
    }

    // ── ASSETMAP + VOLINDEX ──
    assetmap_entries.splice(
        0..0,
        [
            crate::assetmap::AssetMapEntry {
                id: pkl_uuid,
                path: file_name(&pkl_path),
                packing_list: true,
            },
            crate::assetmap::AssetMapEntry {
                id: cpl_uuid,
                path: file_name(&cpl_path),
                packing_list: false,
            },
        ],
    );
    if crate::assetmap::generate_assetmap(&assetmap_entries, &config.output_dir, standard, None)
        != 0
    {
        return Err("failed to write assembled ASSETMAP".into());
    }

    progress.stage(DONE_PERCENT, "done");
    tracing::info!(
        "assembled {} input(s) into {} ({} reels, {} assets)",
        inputs.len(),
        config.output_dir.display(),
        cpl_config.reels.len(),
        pkl_entries.len() - 1
    );
    Ok(())
}

/// One CPL over one shared standard, rate and resolution.
fn check_inputs_agree(inputs: &[InputDcp]) -> Result<(), String> {
    let first = &inputs[0];
    let first_size = (first.geometry.stored_width, first.geometry.stored_height);
    for input in &inputs[1..] {
        if input.standard != first.standard {
            return Err(format!(
                "standard mismatch: {} is {:?} but the first input is {:?}",
                input.dir.display(),
                input.standard,
                first.standard
            ));
        }
        if input.edit_rate != first.edit_rate {
            return Err(format!(
                "frame-rate mismatch: {} is '{}' but the first input is '{}'",
                input.dir.display(),
                input.edit_rate,
                first.edit_rate
            ));
        }
        let size = (input.geometry.stored_width, input.geometry.stored_height);
        if size != first_size {
            return Err(format!(
                "resolution mismatch: {} is {}x{} but the first input is {}x{}",
                input.dir.display(),
                size.0,
                size.1,
                first_size.0,
                first_size.1
            ));
        }
    }
    Ok(())
}

/// Copy the source track files byte for byte, reporting progress by bytes.
fn copy_tracks(
    copies: &[TrackCopy],
    output_dir: &Path,
    progress: &dyn ProgressSink,
) -> Result<
    (
        Vec<crate::pkl::PklEntry>,
        Vec<crate::assetmap::AssetMapEntry>,
    ),
    String,
> {
    let total_bytes = copies
        .iter()
        .map(|copy| file_size(&copy.source))
        .sum::<Result<u64, String>>()?;
    let span = u64::from(COPY_END_PERCENT - COPY_START_PERCENT);
    let mut copied_bytes = 0u64;
    let mut pkl_entries = Vec::with_capacity(copies.len());
    let mut assetmap_entries = Vec::with_capacity(copies.len());
    for copy in copies {
        if progress.cancelled() {
            return Err(crate::dcp::CANCELLED_MESSAGE.into());
        }
        let percent = u64::from(COPY_START_PERCENT) + span * copied_bytes / total_bytes.max(1);
        progress.stage(percent as u32, &format!("copying {}", copy.name));
        let destination = output_dir.join(&copy.name);
        copied_bytes += std::fs::copy(&copy.source, &destination).map_err(|e| {
            format!(
                "cannot copy {} -> {}: {e}",
                copy.source.display(),
                destination.display()
            )
        })?;
        let (hash, size) = match &copy.recorded {
            Some(recorded) => recorded.clone(),
            None => (
                crate::hash::hash_file(&destination)?,
                file_size(&destination)?,
            ),
        };
        pkl_entries.push(crate::pkl::PklEntry {
            id: copy.id.clone(),
            asset_type: "application/mxf".into(),
            file: destination,
            hash,
            size,
        });
        assetmap_entries.push(crate::assetmap::AssetMapEntry {
            id: copy.id.clone(),
            path: copy.name.clone(),
            packing_list: false,
        });
    }
    Ok((pkl_entries, assetmap_entries))
}

fn file_size(path: &Path) -> Result<u64, String> {
    std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| format!("cannot read {}: {e}", path.display()))
}

/// What the item reels have to match, read off the first input.
fn item_format(
    first: &InputDcp,
    sound_track: Option<&Path>,
) -> Result<crate::library_reel::JobFormat, String> {
    let (fps, rate_denominator) = parse_edit_rate(&first.edit_rate);
    if rate_denominator != 1 {
        return Err(format!(
            "library items need a whole frame rate, and {} runs at {}",
            first.dir.display(),
            first.edit_rate
        ));
    }
    Ok(crate::library_reel::JobFormat {
        fps,
        geometry: first.geometry,
        sound: sound_track
            .map(crate::library_reel::job_sound_from_track_file)
            .transpose()?,
    })
}

/// The language the sound track's soundfield group declares, when it has one.
fn spoken_language(track: &Path) -> Option<String> {
    let mut reader = asdcplib::pcm::MxfReader::new();
    reader.open_read(&track.to_string_lossy()).ok()?;
    reader
        .mca_label_subdescriptors()
        .ok()?
        .into_iter()
        .filter(|label| label.kind == asdcplib::pcm::McaLabelKind::SoundfieldGroup)
        .find_map(|label| label.spoken_language)
}

/// Parse one input DCP: exactly one CPL, its reels, picture geometry, edit rate
/// and composition identity. Fails loud on an encrypted CPL (a new CPL id would
/// break its KDM binding).
fn parse_input(dir: &Path) -> Result<InputDcp, String> {
    if !dir.is_dir() {
        return Err("not a directory".into());
    }
    let cpls: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_cpl_path(p))
        .collect();
    if cpls.is_empty() {
        return Err("no CPL found".into());
    }
    if cpls.len() > 1 {
        return Err("has multiple CPLs; assemble takes single-composition DCPs".into());
    }
    let cpl_path = &cpls[0];
    let cpl = std::fs::read_to_string(cpl_path).map_err(|e| e.to_string())?;
    if cpl.contains("<KeyId>") {
        return Err(
            "input is encrypted; a new CPL id breaks the KDM's CPL binding, so assemble refuses \
             encrypted DCPs. Decrypt first (decrypt command) or assemble the plaintext masters"
                .into(),
        );
    }
    let standard = detect_standard(&cpls);
    let reels = get_timeline(cpl_path);
    if reels.is_empty() {
        return Err("CPL has no reels".into());
    }
    let geometry = picture_geometry(&cpl, Path::new(&reels[0].picture_file))?;
    let edit_rate = reels[0].edit_rate.clone();
    let (ratings, content_version_label) = composition_identity(&cpl)?;
    Ok(InputDcp {
        dir: dir.to_path_buf(),
        standard,
        reels,
        geometry,
        edit_rate,
        ratings,
        content_version_label,
    })
}

/// Stored size from the picture track's descriptor, active area from the CPL.
fn picture_geometry(
    cpl_xml: &str,
    picture_file: &Path,
) -> Result<crate::cpl::PictureGeometry, String> {
    let mut reader = asdcplib::jp2k::MxfReader::new();
    reader
        .open_read(&picture_file.to_string_lossy())
        .map_err(|e| {
            format!(
                "cannot open the picture track {}: {e}",
                picture_file.display()
            )
        })?;
    let descriptor = reader.picture_descriptor().map_err(|e| {
        format!(
            "cannot read the picture descriptor of {}: {e}",
            picture_file.display()
        )
    })?;
    let (stored_width, stored_height) = (descriptor.stored_width, descriptor.stored_height);
    let (active_width, active_height) =
        crate::cpl::active_area_from_cpl(cpl_xml).unwrap_or((stored_width, stored_height));
    crate::cpl::check_active_area_fits(stored_width, stored_height, active_width, active_height)?;
    Ok(crate::cpl::PictureGeometry {
        stored_width,
        stored_height,
        active_width,
        active_height,
    })
}

/// The CPL's RatingList and ContentVersion label.
fn composition_identity(cpl_xml: &str) -> Result<(Vec<Rating>, Option<String>), String> {
    let doc = roxmltree::Document::parse(cpl_xml).map_err(|e| format!("CPL: {e}"))?;
    let child_text = |node: roxmltree::Node, name: &str| -> Option<String> {
        node.children()
            .find(|n| n.has_tag_name(name))
            .and_then(|n| n.text())
            .map(|t| t.trim().to_string())
    };
    let root = doc.root_element();
    let child = |name: &str| root.children().find(|n| n.has_tag_name(name));
    let ratings = child("RatingList")
        .map(|list| {
            list.children()
                .filter(|n| n.has_tag_name("Rating"))
                .map(|rating| Rating {
                    agency: child_text(rating, "Agency").unwrap_or_default(),
                    label: child_text(rating, "Label").unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    let title = child_text(root, "ContentTitleText");
    // a label equal to the source's title is the writer's default, not a version
    let content_version_label = child("ContentVersion")
        .and_then(|version| child_text(version, "LabelText"))
        .filter(|label| Some(label) != title.as_ref());
    Ok((ratings, content_version_label))
}

/// id -> (hash, size) across every PKL in the directory.
fn parse_pkls(dir: &Path) -> HashMap<String, (String, u64)> {
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return map;
    };
    for entry in entries.flatten() {
        let name = file_name(&entry.path());
        if !(name.starts_with("PKL") && name.ends_with(".xml")) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for block in content.split("<Asset>").skip(1) {
            let block = block.split("</Asset>").next().unwrap_or("");
            let id = tag(block, "Id").map(|v| v.replace("urn:uuid:", ""));
            let hash = tag(block, "Hash");
            let (Some(id), Some(hash)) = (id, hash) else {
                continue;
            };
            let size = tag(block, "Size").and_then(|s| s.parse().ok()).unwrap_or(0);
            map.insert(id, (hash, size));
        }
    }
    map
}

/// Pick a unique output filename, appending a short id suffix on collision.
fn unique_name(filename: &str, id: &str, used: &mut HashSet<String>) -> String {
    if used.insert(filename.to_string()) {
        return filename.to_string();
    }
    let (stem, ext) = match filename.rsplit_once('.') {
        Some((s, e)) => (s, format!(".{e}")),
        None => (filename, String::new()),
    };
    let short = &id[..id.len().min(8)];
    let mut candidate = format!("{stem}_{short}{ext}");
    let mut n = 1;
    while !used.insert(candidate.clone()) {
        candidate = format!("{stem}_{short}_{n}{ext}");
        n += 1;
    }
    candidate
}

fn parse_edit_rate(s: &str) -> (u32, u32) {
    let mut it = s.split_whitespace();
    let num = it.next().and_then(|v| v.parse().ok()).unwrap_or(24);
    let den = it.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    (num, den)
}
