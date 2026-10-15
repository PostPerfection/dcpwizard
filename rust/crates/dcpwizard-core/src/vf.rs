use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Version File DCP configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VfConfig {
    pub ov_dir: PathBuf,
    pub vf_dir: PathBuf,
    pub title: String,
    pub replacement_reels: Vec<ReplacementReel>,
    /// Language code for wrapped subtitle tracks (default "en").
    pub subtitle_language: String,
    /// How the wrapped timed text is rendered: the font to embed and the
    /// placement. Without a font here the machine's own faces are used.
    pub subtitle_opts: crate::subtitle::SubtitleOptions,
    pub signer: Option<crate::package_signature::PackageSigner>,
}

/// A reel in the VF that replaces one or more OV essence tracks. A track is
/// raw essence (J2K frames for picture, WAV for sound, SRT/SMPTE XML for
/// subtitle) that gets wrapped, or an already-wrapped `.mxf`. Reels with no
/// replacement are referenced from the OV.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplacementReel {
    pub reel_number: u32,
    pub picture: Option<PathBuf>,
    pub sound: Option<PathBuf>,
    /// Subtitle to add or replace on this reel (SRT converted, SMPTE XML wrapped
    /// unchanged). Both --add-subtitle and --replace-subtitle land here.
    pub subtitle: Option<PathBuf>,
    /// Closed caption to add or replace on this reel. Same input formats as
    /// `subtitle`; emitted under the ST 429-12 ClosedCaption role.
    #[serde(default)]
    pub ccap: Option<PathBuf>,
}

/// A new MXF that physically ships in the VF (registered in PKL + ASSETMAP).
struct NewAsset {
    id: String,
    filename: String,
    hash: String,
    size: u64,
    duration: u64,
    key: Option<crate::encrypt::ContentKey>,
}

impl NewAsset {
    fn key_id(&self) -> Option<String> {
        self.key.as_ref().map(|k| k.key_id.clone())
    }

    fn timed_text_track(&self, language: &str) -> TimedTextTrack {
        TimedTextTrack {
            id: self.id.clone(),
            duration: self.duration,
            hash: Some(self.hash.clone()),
            key_id: self.key_id(),
            language: Some(language.to_string()),
        }
    }
}

// a subtitle or closed caption the VF reel lists, new or referenced from the OV
struct TimedTextTrack {
    id: String,
    duration: u64,
    hash: Option<String>,
    key_id: Option<String>,
    language: Option<String>,
}

fn mint_key(
    key_type: Option<crate::encrypt::KeyType>,
    asset_id: &str,
) -> Result<Option<crate::encrypt::GeneratedKey>, ()> {
    let Some(key_type) = key_type else {
        return Ok(None);
    };
    crate::encrypt::generate_content_key(key_type, asset_id)
        .map(Some)
        .map_err(|e| tracing::error!("content key generation failed: {e}"))
}

/// Create a Version File DCP that references the Original Version.
///
/// Unchanged reels reference the OV's real asset ids; the VF ships only the new
/// MXFs plus its own CPL/PKL/ASSETMAP. Replaced tracks are wrapped (or copied if
/// already MXF) and registered under their real asset id in all three files.
// the folder the VF is written to: vf_dir plus the title, or <OV title>_VF
pub fn vf_package_dir(config: &VfConfig) -> Result<PathBuf, String> {
    crate::package_dir::package_dir(&config.vf_dir, &vf_title(config)?)
}

pub fn new_vf_package_dir(config: &VfConfig) -> Result<PathBuf, String> {
    crate::package_dir::new_package_dir(&config.vf_dir, &vf_title(config)?)
}

fn vf_title(config: &VfConfig) -> Result<String, String> {
    if !config.title.is_empty() {
        return Ok(config.title.clone());
    }
    let ov_cpls = crate::multi_cpl::list_cpls(&config.ov_dir);
    let Some(ov_cpl) = ov_cpls.first() else {
        return Err(format!(
            "No CPL found in OV directory {}",
            config.ov_dir.display()
        ));
    };
    Ok(format!("{}_VF", ov_cpl.content_title))
}

pub enum OvKeySource {
    KeysFile(PathBuf),
    Kdm {
        kdm: PathBuf,
        recipient_key: PathBuf,
    },
}

impl OvKeySource {
    fn describe(&self) -> String {
        match self {
            OvKeySource::KeysFile(path) => format!("OV keys file {}", path.display()),
            OvKeySource::Kdm { kdm, .. } => format!("OV KDM {}", kdm.display()),
        }
    }
}

#[derive(Default)]
pub struct VfKeyOptions {
    pub key_out: Option<PathBuf>,
    pub ov_keys: Option<OvKeySource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OvTrack {
    Picture,
    Sound,
    Subtitle,
    ClosedCaption,
    AuxData,
}

impl OvTrack {
    fn key_type(self) -> crate::encrypt::KeyType {
        use crate::encrypt::KeyType;
        match self {
            OvTrack::Picture => KeyType::Mdik,
            OvTrack::Sound => KeyType::Mdak,
            OvTrack::Subtitle | OvTrack::ClosedCaption => KeyType::Mdsk,
            OvTrack::AuxData => KeyType::Mdek,
        }
    }

    fn name(self) -> &'static str {
        match self {
            OvTrack::Picture => "picture",
            OvTrack::Sound => "sound",
            OvTrack::Subtitle => "subtitle",
            OvTrack::ClosedCaption => "closed caption",
            OvTrack::AuxData => "aux data",
        }
    }
}

// an encrypted OV track file the VF CPL points at
struct EncryptedOvReference {
    track: OvTrack,
    asset_id: String,
    key_id: uuid::Uuid,
}

struct OfferedOvKey {
    key_id: uuid::Uuid,
    key_type: Option<crate::encrypt::KeyType>,
    asset_uuid: Option<String>,
    content_key_hex: String,
}

fn bare_uuid(text: &str) -> String {
    text.trim()
        .trim_start_matches("urn:uuid:")
        .to_ascii_lowercase()
}

fn key_ids_by_asset(
    cpl_xml: &str,
) -> Result<std::collections::HashMap<String, uuid::Uuid>, String> {
    let doc =
        roxmltree::Document::parse(cpl_xml).map_err(|e| format!("cannot read the OV CPL: {e}"))?;
    let child_text = |node: roxmltree::Node, name: &str| {
        node.children()
            .find(|c| c.has_tag_name(name))
            .and_then(|c| c.text())
            .map(bare_uuid)
    };
    let mut key_ids = std::collections::HashMap::new();
    for asset in doc.descendants().filter(|n| n.is_element()) {
        let (Some(asset_id), Some(key_id)) = (child_text(asset, "Id"), child_text(asset, "KeyId"))
        else {
            continue;
        };
        let key_id = uuid::Uuid::parse_str(&key_id)
            .map_err(|e| format!("OV CPL asset {asset_id} has KeyId '{key_id}': {e}"))?;
        key_ids.insert(asset_id, key_id);
    }
    Ok(key_ids)
}

fn offered_ov_keys(source: &OvKeySource, ov_cpl_id: &str) -> Result<Vec<OfferedOvKey>, String> {
    match source {
        OvKeySource::KeysFile(path) => crate::encrypt::KeyBundle::read_for_cpl(path, ov_cpl_id)?
            .keys
            .into_iter()
            .map(|key| {
                let key_id = uuid::Uuid::parse_str(&bare_uuid(&key.key_id)).map_err(|e| {
                    format!(
                        "keys file {} has KeyId '{}': {e}",
                        path.display(),
                        key.key_id
                    )
                })?;
                Ok(OfferedOvKey {
                    key_id,
                    key_type: Some(key.key_type),
                    asset_uuid: Some(bare_uuid(&key.asset_uuid)).filter(|id| !id.is_empty()),
                    content_key_hex: key.content_key_hex.clone(),
                })
            })
            .collect(),
        OvKeySource::Kdm { kdm, recipient_key } => {
            let unwrapped =
                postkit::certificate::unwrap_kdm_file(kdm, recipient_key).map_err(|e| {
                    format!(
                        "KDM {} does not decrypt with the private key {}: {e}",
                        kdm.display(),
                        recipient_key.display()
                    )
                })?;
            let want = bare_uuid(ov_cpl_id);
            if unwrapped.cpl_id.to_string() != want {
                return Err(format!(
                    "KDM {} is for CPL {}, not the OV's CPL {want}",
                    kdm.display(),
                    unwrapped.cpl_id
                ));
            }
            Ok(unwrapped
                .keys
                .iter()
                .map(|key| OfferedOvKey {
                    key_id: key.key_id,
                    key_type: key
                        .key_type
                        .and_then(|t| crate::encrypt::KeyType::from_smpte(&t)),
                    asset_uuid: None,
                    content_key_hex: crate::encrypt::hex_encode(key.content_key()),
                })
                .collect())
        }
    }
}

// matched by the KeyId the OV CPL lists, never by what the key source claims
fn ov_keys_for_references(
    source: &OvKeySource,
    ov_cpl_id: &str,
    references: &[EncryptedOvReference],
) -> Result<Vec<crate::encrypt::ContentKey>, String> {
    let offered = offered_ov_keys(source, ov_cpl_id)?;
    let mut chosen: Vec<crate::encrypt::ContentKey> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for reference in references {
        if let Some(conflict) = offered.iter().find(|key| {
            key.asset_uuid.as_deref() == Some(reference.asset_id.as_str())
                && key.key_id != reference.key_id
        }) {
            return Err(format!(
                "{} gives the OV {} asset {} the KeyId {}, but the OV CPL lists {}",
                source.describe(),
                reference.track.name(),
                reference.asset_id,
                conflict.key_id,
                reference.key_id
            ));
        }
        let Some(key) = offered.iter().find(|key| key.key_id == reference.key_id) else {
            missing.push(reference.key_id.to_string());
            continue;
        };
        let wanted_type = reference.track.key_type();
        if let Some(key_type) = key.key_type
            && key_type != wanted_type
        {
            return Err(format!(
                "{} holds KeyId {} as a {} key, but the OV CPL uses it for the {} asset {}",
                source.describe(),
                key.key_id,
                String::from_utf8_lossy(&key_type.as_smpte()),
                reference.track.name(),
                reference.asset_id
            ));
        }
        let key_id = reference.key_id.to_string();
        if chosen.iter().any(|held| held.key_id == key_id) {
            continue;
        }
        chosen.push(crate::encrypt::ContentKey {
            key_type: wanted_type,
            key_id,
            asset_uuid: reference.asset_id.clone(),
            content_key_hex: key.content_key_hex.clone(),
        });
    }
    if !missing.is_empty() {
        return Err(format!(
            "{} has no key for the OV KeyIds the VF references: {}",
            source.describe(),
            missing.join(", ")
        ));
    }
    Ok(chosen)
}

fn write_keys_file(path: &Path, bundle: &crate::encrypt::KeyBundle) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create key-out directory: {e}"))?;
    }
    bundle
        .write(path)
        .map_err(|e| format!("Failed to write keys file: {e}"))?;
    tracing::warn!(
        "Wrote content keys to {}. This file holds the plaintext AES keys: keep it secret, \
         do not ship it in the DCP, and use it only to build KDMs.",
        path.display()
    );
    Ok(())
}

pub fn create_vf(config: &VfConfig) -> i32 {
    match create_vf_with_keys(config, &VfKeyOptions::default()) {
        Ok(_) => 0,
        Err(()) => -1,
    }
}

// Ok holds the OV KeyIds the VF CPL references that the written keys file has no key for
pub fn create_vf_with_keys(config: &VfConfig, keys: &VfKeyOptions) -> Result<Vec<String>, ()> {
    if !config.ov_dir.exists() {
        tracing::error!("OV directory not found: {}", config.ov_dir.display());
        return Err(());
    }
    let vf_dir = match new_vf_package_dir(config) {
        Ok(vf_dir) => vf_dir,
        Err(e) => {
            tracing::error!("{e}");
            return Err(());
        }
    };

    // Read the OV CPL to get reel structure and identity.
    let ov_cpls = crate::multi_cpl::list_cpls(&config.ov_dir);
    let Some(ov_cpl) = ov_cpls.first() else {
        tracing::error!("No CPL found in OV directory");
        return Err(());
    };
    let ov_cpl_path = config.ov_dir.join(&ov_cpl.file_path);
    let ov_timeline = crate::multi_cpl::get_timeline(&ov_cpl_path);
    if ov_timeline.is_empty() {
        tracing::error!("OV CPL has no reels");
        return Err(());
    }

    // Fail loud if a replacement targets a reel the OV doesn't have.
    for rep in &config.replacement_reels {
        if !ov_timeline.iter().any(|e| e.reel_number == rep.reel_number) {
            tracing::error!(
                "replacement targets reel {} which the OV does not have",
                rep.reel_number
            );
            return Err(());
        }
    }

    // A VF that replaces nothing is just a copy: refuse it.
    if !config.replacement_reels.iter().any(|r| {
        r.picture.is_some() || r.sound.is_some() || r.subtitle.is_some() || r.ccap.is_some()
    }) {
        tracing::error!("no replacement essence supplied; nothing to replace");
        return Err(());
    }

    let ov_cpl_content = std::fs::read_to_string(&ov_cpl_path).unwrap_or_default();
    let ov_key_ids = match key_ids_by_asset(&ov_cpl_content) {
        Ok(key_ids) => key_ids,
        Err(e) => {
            tracing::error!("{e}");
            return Err(());
        }
    };
    let ov_identity = match crate::assemble::composition_identity(&ov_cpl_content) {
        Ok(identity) => identity,
        Err(e) => {
            tracing::error!("cannot read the OV CPL: {e}");
            return Err(());
        }
    };

    let replacement_for = |reel_number: u32| {
        config
            .replacement_reels
            .iter()
            .find(|r| r.reel_number == reel_number)
    };
    let mut encrypted_references: Vec<EncryptedOvReference> = Vec::new();
    for entry in &ov_timeline {
        let rep = replacement_for(entry.reel_number);
        let kept = |replacement: Option<&PathBuf>, asset_id: &str| {
            replacement.is_none() && !asset_id.is_empty()
        };
        for (track, kept, asset_id) in [
            (
                OvTrack::Picture,
                kept(
                    rep.and_then(|r| r.picture.as_ref()),
                    &entry.picture_asset_id,
                ),
                &entry.picture_asset_id,
            ),
            (
                OvTrack::Sound,
                kept(rep.and_then(|r| r.sound.as_ref()), &entry.sound_asset_id),
                &entry.sound_asset_id,
            ),
            (
                OvTrack::Subtitle,
                kept(
                    rep.and_then(|r| r.subtitle.as_ref()),
                    &entry.subtitle_asset_id,
                ),
                &entry.subtitle_asset_id,
            ),
            (
                OvTrack::ClosedCaption,
                kept(rep.and_then(|r| r.ccap.as_ref()), &entry.ccap_asset_id),
                &entry.ccap_asset_id,
            ),
            (
                OvTrack::AuxData,
                kept(None, &entry.aux_data_asset_id),
                &entry.aux_data_asset_id,
            ),
        ] {
            let asset_id = bare_uuid(asset_id);
            if let (true, Some(key_id)) = (kept, ov_key_ids.get(&asset_id)) {
                encrypted_references.push(EncryptedOvReference {
                    track,
                    asset_id,
                    key_id: *key_id,
                });
            }
        }
    }
    let encrypted = !encrypted_references.is_empty();

    if !encrypted && (keys.key_out.is_some() || keys.ov_keys.is_some()) {
        tracing::error!(
            "the OV assets this VF references are not encrypted, so there are no keys to write \
             or read: drop --key-out, --ov-keys and --ov-kdm"
        );
        return Err(());
    }
    if encrypted && config.signer.is_none() {
        tracing::error!(
            "{}: the VF references encrypted OV assets",
            crate::preflight::ENCRYPTED_PACKAGE_NEEDS_SIGNER
        );
        return Err(());
    }
    if encrypted && keys.key_out.is_none() {
        tracing::error!(
            "--key-out is required when the OV is encrypted; keys are never written next to the DCP"
        );
        return Err(());
    }
    let ov_content_keys = match keys.ov_keys.as_ref() {
        Some(source) => match ov_keys_for_references(source, &ov_cpl.id, &encrypted_references) {
            Ok(content_keys) => content_keys,
            Err(e) => {
                tracing::error!("{e}");
                return Err(());
            }
        },
        None => Vec::new(),
    };
    let mut missing_ov_key_ids: Vec<String> = Vec::new();
    if keys.ov_keys.is_none() {
        for reference in &encrypted_references {
            let key_id = reference.key_id.to_string();
            if !missing_ov_key_ids.contains(&key_id) {
                missing_ov_key_ids.push(key_id);
            }
        }
    }
    let new_key_type = |key_type: crate::encrypt::KeyType| -> Option<crate::encrypt::KeyType> {
        encrypted.then_some(key_type)
    };

    // prove the signer works before anything is written, so a bad one cannot
    // leave a half-signed package behind
    if let Some(signer) = config.signer.as_ref()
        && let Err(e) = signer.check_usable()
    {
        tracing::error!("unusable signer: {e}");
        return Err(());
    }

    let sub_lang = if config.subtitle_language.is_empty() {
        "en"
    } else {
        &config.subtitle_language
    };

    if let Err(e) = std::fs::create_dir_all(&vf_dir) {
        tracing::error!("Failed to create VF directory: {e}");
        return Err(());
    }

    let standard = if ov_cpl_content.contains("digicine.com") {
        crate::Standard::Interop
    } else {
        crate::Standard::Smpte
    };
    // vf inherits the ov's picture dimensions (reel coherence keeps them uniform).
    let (pic_w, pic_h) = parse_screen_aspect(&ov_cpl_content);
    // keep the OV's masking: without this a VF over a letterboxed OV would
    // declare the whole raster active
    let (active_w, active_h) =
        crate::cpl::active_area_from_cpl(&ov_cpl_content).unwrap_or((pic_w, pic_h));
    let ov_markers = crate::markers::markers_from_cpl(&ov_cpl_content);
    let ov_key_id = |asset_id: &str| ov_key_ids.get(&bare_uuid(asset_id)).map(|k| k.to_string());

    let mut cpl_reels: Vec<crate::cpl::CplReel> = Vec::new();
    let mut new_assets: Vec<NewAsset> = Vec::new();
    let mut main_sound_track: Option<PathBuf> = None;
    let mut sound_replaced = false;

    for (reel_index, entry) in ov_timeline.iter().enumerate() {
        let rep = replacement_for(entry.reel_number);
        let (edit_num, edit_den) = parse_edit_rate(&entry.edit_rate);

        // Picture: always present in a DCP reel.
        let (picture_id, picture_duration, picture_key_id) =
            match rep.and_then(|r| r.picture.as_ref()) {
                Some(input) => {
                    let Some(a) = prepare_asset(
                        input,
                        "picture",
                        crate::mxf_wrap::MxfType::J2kPicture,
                        edit_num,
                        entry.duration_frames,
                        new_key_type(crate::encrypt::KeyType::Mdik),
                        &vf_dir,
                    ) else {
                        return Err(());
                    };
                    if !replacement_fits_reel(entry, "picture", input, &a) {
                        return Err(());
                    }
                    let out = (a.id.clone(), a.duration, a.key_id());
                    new_assets.push(a);
                    out
                }
                None => (
                    entry.picture_asset_id.clone(),
                    entry.duration_frames,
                    ov_key_id(&entry.picture_asset_id),
                ),
            };

        // Sound: from a replacement, or referenced from the OV, or absent.
        let sound = match rep.and_then(|r| r.sound.as_ref()) {
            Some(input) => {
                let Some(a) = prepare_asset(
                    input,
                    "sound",
                    crate::mxf_wrap::MxfType::PcmAudio,
                    edit_num,
                    entry.duration_frames,
                    new_key_type(crate::encrypt::KeyType::Mdak),
                    &vf_dir,
                ) else {
                    return Err(());
                };
                if !replacement_fits_reel(entry, "sound", input, &a) {
                    return Err(());
                }
                sound_replaced = true;
                let out = Some((a.id.clone(), a.duration, a.key_id()));
                if main_sound_track.is_none() {
                    main_sound_track = Some(vf_dir.join(&a.filename));
                }
                new_assets.push(a);
                out
            }
            None if !entry.sound_asset_id.is_empty() => {
                if main_sound_track.is_none() {
                    main_sound_track = Some(PathBuf::from(&entry.sound_file));
                }
                Some((
                    entry.sound_asset_id.clone(),
                    entry.duration_frames,
                    ov_key_id(&entry.sound_asset_id),
                ))
            }
            None => None,
        };

        let ov_timed_text = |asset_id: &str, language: &str| {
            (!asset_id.is_empty()).then(|| TimedTextTrack {
                id: asset_id.to_string(),
                duration: entry.duration_frames,
                hash: None,
                key_id: ov_key_id(asset_id),
                language: (!language.is_empty()).then(|| language.to_string()),
            })
        };

        // Subtitle: a new track added or replaced on this reel, else the OV's.
        let subtitle = match rep.and_then(|r| r.subtitle.as_ref()) {
            Some(input) => {
                let Some(a) = prepare_timed_text(
                    input,
                    "subtitle",
                    sub_lang,
                    edit_num,
                    entry.duration_frames,
                    &config.subtitle_opts,
                    new_key_type(crate::encrypt::KeyType::Mdsk),
                    &vf_dir,
                ) else {
                    return Err(());
                };
                let out = Some(a.timed_text_track(sub_lang));
                new_assets.push(a);
                out
            }
            None => ov_timed_text(&entry.subtitle_asset_id, &entry.subtitle_language),
        };

        // Closed caption: same handling, emitted under the ST 429-12 ClosedCaption role.
        let ccap = match rep.and_then(|r| r.ccap.as_ref()) {
            Some(input) => {
                let Some(a) = prepare_timed_text(
                    input,
                    "ccap",
                    sub_lang,
                    edit_num,
                    entry.duration_frames,
                    &config.subtitle_opts.for_closed_caption(),
                    new_key_type(crate::encrypt::KeyType::Mdsk),
                    &vf_dir,
                ) else {
                    return Err(());
                };
                let out = Some(a.timed_text_track(sub_lang));
                new_assets.push(a);
                out
            }
            None => ov_timed_text(&entry.ccap_asset_id, &entry.ccap_language),
        };

        let aux_data = (!entry.aux_data_asset_id.is_empty()).then(|| crate::cpl::AuxData {
            id: entry.aux_data_asset_id.clone(),
            edit_rate_num: edit_num,
            edit_rate_den: edit_den,
            duration: entry.duration_frames,
            entry_point: 0,
            key_id: ov_key_id(&entry.aux_data_asset_id),
            data_type: entry.aux_data_type.clone(),
        });

        cpl_reels.push(crate::cpl::CplReel {
            reel_id: uuid::Uuid::new_v4().to_string(),
            picture_id,
            picture_width: pic_w,
            picture_height: pic_h,
            picture_active_width: active_w,
            picture_active_height: active_h,
            picture_edit_rate_num: edit_num,
            picture_edit_rate_den: edit_den,
            picture_duration,
            picture_entry_point: 0,
            picture_key_id,
            sound_id: sound.as_ref().map(|s| s.0.clone()),
            sound_edit_rate_num: edit_num,
            sound_edit_rate_den: edit_den,
            sound_duration: sound.as_ref().map(|s| s.1).unwrap_or(0),
            sound_entry_point: 0,
            sound_key_id: sound.as_ref().and_then(|s| s.2.clone()),
            subtitle_id: subtitle.as_ref().map(|s| s.id.clone()),
            subtitle_edit_rate_num: if subtitle.is_some() { edit_num } else { 0 },
            subtitle_edit_rate_den: if subtitle.is_some() { edit_den } else { 0 },
            subtitle_duration: subtitle.as_ref().map(|s| s.duration).unwrap_or(0),
            subtitle_entry_point: 0,
            subtitle_language: subtitle.as_ref().and_then(|s| s.language.clone()),
            subtitle_hash: subtitle.as_ref().and_then(|s| s.hash.clone()),
            subtitle_key_id: subtitle.as_ref().and_then(|s| s.key_id.clone()),
            ccap_id: ccap.as_ref().map(|c| c.id.clone()),
            ccap_edit_rate_num: if ccap.is_some() { edit_num } else { 0 },
            ccap_edit_rate_den: if ccap.is_some() { edit_den } else { 0 },
            ccap_duration: ccap.as_ref().map(|c| c.duration).unwrap_or(0),
            ccap_entry_point: 0,
            ccap_language: ccap.as_ref().and_then(|c| c.language.clone()),
            ccap_hash: ccap.as_ref().and_then(|c| c.hash.clone()),
            ccap_key_id: ccap.as_ref().and_then(|c| c.key_id.clone()),
            stereoscopic: false,
            aux_data,
            markers: ov_markers.get(reel_index).cloned().unwrap_or_default(),
            ..Default::default()
        });
    }
    // Bv2.1 wants FFOC and LFOC even when the OV has no markers
    if cpl_reels.iter().all(|reel| reel.markers.is_empty()) {
        crate::cpl::apply_default_markers(&mut cpl_reels);
    }

    let title = if config.title.is_empty() {
        format!("{}_VF", ov_cpl.content_title)
    } else {
        config.title.clone()
    };
    let content_kind = if ov_cpl.content_kind.is_empty() {
        "feature".to_string()
    } else {
        ov_cpl.content_kind.clone()
    };

    // the VF can replace the sound, so the OV CPL's block may not describe it
    let main_sound = match main_sound_track {
        Some(ref track) if standard == crate::Standard::Smpte => {
            match crate::cpl::main_sound_from_track_file(track) {
                Ok(sound) => Some(sound),
                Err(e) => {
                    tracing::error!("{e}");
                    return Err(());
                }
            }
        }
        _ => None,
    };
    // the sign-language video is packed in the OV sound track
    let sign_language = crate::cpl::sign_language_from_cpl(&ov_cpl_content);
    if sign_language.is_some() && sound_replaced {
        tracing::warn!(
            "the OV's sign-language video is in the sound it replaces, so the VF does not declare it"
        );
    }

    // ── Write CPL via the shared postkit writer, then mark it supplemental ──
    let cpl_uuid = uuid::Uuid::new_v4().to_string();
    let cpl_path = vf_dir.join(format!("CPL_{cpl_uuid}.xml"));
    let cpl_config = crate::cpl::CplConfig {
        title,
        content_kind,
        rating: String::new(),
        ratings: ov_identity.ratings,
        reels: cpl_reels,
        standard,
        main_sound,
        sign_language: sign_language.filter(|_| !sound_replaced),
        ..Default::default()
    };
    if crate::cpl::generate_cpl(&cpl_config, &cpl_uuid, &cpl_path) != 0 {
        tracing::error!("Failed to generate VF CPL");
        return Err(());
    }

    if !crate::package_signature::sign_if_configured(config.signer.as_ref(), &cpl_path, "VF CPL") {
        return Err(());
    }

    // ── PKL: the CPL plus every new MXF ────────────────────────────────────
    let pkl_uuid = uuid::Uuid::new_v4().to_string();
    let cpl_hash = crate::hash::hash_file(&cpl_path).unwrap_or_default();
    let cpl_size = std::fs::metadata(&cpl_path).map(|m| m.len()).unwrap_or(0);
    let mut pkl_entries = vec![crate::pkl::PklEntry {
        id: cpl_uuid.clone(),
        asset_type: "text/xml".into(),
        file: cpl_path.clone(),
        hash: cpl_hash,
        size: cpl_size,
    }];
    for a in &new_assets {
        pkl_entries.push(crate::pkl::PklEntry {
            id: a.id.clone(),
            asset_type: "application/mxf".into(),
            file: vf_dir.join(&a.filename),
            hash: a.hash.clone(),
            size: a.size,
        });
    }
    let pkl_path = vf_dir.join(format!("PKL_{pkl_uuid}.xml"));
    if crate::pkl::generate_pkl(&pkl_entries, &pkl_uuid, standard, None, &pkl_path) != 0 {
        tracing::error!("Failed to generate VF PKL");
        return Err(());
    }
    if !crate::package_signature::sign_if_configured(config.signer.as_ref(), &pkl_path, "VF PKL") {
        return Err(());
    }

    // ── ASSETMAP: PKL, CPL, and every new MXF ──────────────────────────────
    let mut am_entries = vec![
        crate::assetmap::AssetMapEntry {
            id: pkl_uuid,
            path: file_name(&pkl_path),
            packing_list: true,
        },
        crate::assetmap::AssetMapEntry {
            id: cpl_uuid.clone(),
            path: file_name(&cpl_path),
            packing_list: false,
        },
    ];
    for a in &new_assets {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: a.id.clone(),
            path: a.filename.clone(),
            packing_list: false,
        });
    }
    if crate::assetmap::generate_assetmap(&am_entries, &vf_dir, standard, None) != 0 {
        tracing::error!("Failed to generate VF ASSETMAP");
        return Err(());
    }

    if let Some(keys_path) = keys.key_out.as_ref().filter(|_| encrypted) {
        let bundle = crate::encrypt::KeyBundle {
            cpl_id: cpl_uuid,
            keys: new_assets
                .iter()
                .filter_map(|a| a.key.clone())
                .chain(ov_content_keys)
                .collect(),
        };
        if let Err(e) = write_keys_file(keys_path, &bundle) {
            tracing::error!("{e}");
            return Err(());
        }
    }

    tracing::info!(
        "Created VF DCP at {} ({} new asset(s))",
        vf_dir.display(),
        new_assets.len()
    );
    Ok(missing_ov_key_ids)
}

/// Read the AssetUUID an already-wrapped MXF carries, so a copied track file is
/// listed in the CPL/PKL/ASSETMAP under the id it actually holds, and its
/// length in edit units.
fn wrapped_asset_uuid_and_duration(
    path: &Path,
    mxf_type: crate::mxf_wrap::MxfType,
) -> Option<(uuid::Uuid, u64)> {
    let file = path.to_string_lossy().to_string();
    let (info, duration) = match mxf_type {
        crate::mxf_wrap::MxfType::J2kPicture => {
            let mut reader = asdcplib::jp2k::MxfReader::new();
            reader.open_read(&file).ok()?;
            let duration = reader.picture_descriptor().ok()?.container_duration;
            (reader.writer_info().ok()?, duration)
        }
        crate::mxf_wrap::MxfType::PcmAudio => {
            let mut reader = asdcplib::pcm::MxfReader::new();
            reader.open_read(&file).ok()?;
            let duration = reader.audio_descriptor().ok()?.container_duration;
            (reader.writer_info().ok()?, duration)
        }
        crate::mxf_wrap::MxfType::TimedText => {
            let mut reader = asdcplib::timed_text::MxfReader::new();
            reader.open_read(&file).ok()?;
            let duration = reader.descriptor().ok()?.container_duration;
            (reader.writer_info().ok()?, duration)
        }
        crate::mxf_wrap::MxfType::Atmos => {
            let mut reader = asdcplib::atmos::MxfReader::new();
            reader.open_read(&file).ok()?;
            let duration = reader.atmos_descriptor().ok()?.container_duration;
            (reader.writer_info().ok()?, duration)
        }
    };
    Some((uuid::Uuid::from_bytes(info.asset_uuid), u64::from(duration)))
}

fn replacement_fits_reel(
    reel: &crate::multi_cpl::TimelineEntry,
    track: &str,
    input: &Path,
    replacement: &NewAsset,
) -> bool {
    if replacement.duration == reel.duration_frames {
        return true;
    }
    tracing::error!(
        "reel {} is {} frames long, but the replacement {track} {} is {} frames",
        reel.reel_number,
        reel.duration_frames,
        input.display(),
        replacement.duration
    );
    false
}

/// Wrap raw essence (or copy an already-wrapped MXF) into the VF directory and
/// return its real asset id, hash, and size. `None` on any failure (logged).
fn prepare_asset(
    input: &Path,
    prefix: &str,
    mxf_type: crate::mxf_wrap::MxfType,
    fps: u32,
    fallback_duration: u64,
    key_type: Option<crate::encrypt::KeyType>,
    vf_dir: &Path,
) -> Option<NewAsset> {
    if !input.exists() {
        tracing::error!("replacement essence not found: {}", input.display());
        return None;
    }

    let is_mxf = input.is_file()
        && input
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("mxf"));

    if is_mxf && key_type.is_some() {
        tracing::error!(
            "the VF references encrypted OV assets, so its {prefix} has to be encrypted too: \
             give raw essence instead of the wrapped MXF {}",
            input.display()
        );
        return None;
    }

    let (id, filename, duration, key) = if is_mxf {
        // Already wrapped: copy verbatim under the id the MXF itself carries,
        // so the CPL/PKL/ASSETMAP entries match what the file holds.
        let Some((id, duration)) = wrapped_asset_uuid_and_duration(input, mxf_type) else {
            tracing::error!("cannot read the asset id of MXF {}", input.display());
            return None;
        };
        let filename = format!("{prefix}_{id}.mxf");
        let id = id.to_string();
        if let Err(e) = std::fs::copy(input, vf_dir.join(&filename)) {
            tracing::error!("Failed to copy MXF {}: {e}", input.display());
            return None;
        }
        (id, filename, duration, None)
    } else {
        // Raw essence: mint the id, name the file with it and wrap the MXF under it.
        let id = uuid::Uuid::new_v4();
        let key = mint_key(key_type, &id.to_string()).ok()?;
        let filename = format!("{prefix}_{id}.mxf");
        let packaged_sound = vf_dir.join(format!(".dcpwizard_audio_{id}.wav"));
        let input_path = match mxf_type {
            crate::mxf_wrap::MxfType::PcmAudio => {
                match crate::mxf_wrap::prepare_packaged_channels(
                    input,
                    &packaged_sound,
                    crate::mxf_wrap::AudioInputOrder::default(),
                    None,
                ) {
                    Ok(true) => packaged_sound.clone(),
                    Ok(false) => input.to_path_buf(),
                    Err(e) => {
                        tracing::error!("audio preparation failed: {e}");
                        return None;
                    }
                }
            }
            _ => input.to_path_buf(),
        };
        let wrap_config = crate::mxf_wrap::MxfWrapConfig {
            input_path,
            output_mxf: vf_dir.join(&filename),
            mxf_type,
            frame_rate: fps,
            encryption: key.as_ref().map(crate::reel::mxf_enc),
            mca_config: None,
            asset_uuid: Some(*id.as_bytes()),
        };
        let wrapped = crate::mxf_wrap::wrap_mxf_result(&wrap_config);
        let _ = std::fs::remove_file(&packaged_sound);
        let tf = wrapped?;
        let duration = if tf.duration > 0 {
            tf.duration
        } else {
            fallback_duration
        };
        (tf.uuid, filename, duration, key.map(|k| k.info))
    };

    let path = vf_dir.join(&filename);
    let hash = match crate::hash::hash_file(&path) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash {}: {e}", path.display());
            return None;
        }
    };
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Some(NewAsset {
        id,
        filename,
        hash,
        size,
        duration,
        key,
    })
}

/// Wrap a timed-text track (subtitle or closed caption) into the VF: SRT is
/// converted to ST 428-7 DCST first, supplied SMPTE XML is wrapped unchanged.
/// `prefix` names the loose/wrapped files ("subtitle" or "ccap"). Returns the
/// wrapped track's real asset id, hash, size, and duration. `None` on failure.
#[allow(clippy::too_many_arguments)]
fn prepare_timed_text(
    input: &Path,
    prefix: &str,
    lang: &str,
    fps: u32,
    fallback_duration: u64,
    opts: &crate::subtitle::SubtitleOptions,
    key_type: Option<crate::encrypt::KeyType>,
    vf_dir: &Path,
) -> Option<NewAsset> {
    if !input.exists() {
        tracing::error!("{prefix} not found: {}", input.display());
        return None;
    }
    let is_xml = input
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xml"));

    // SRT gets converted to a temp DCST; supplied XML is wrapped as-is.
    let mut temp_dcst: Option<PathBuf> = None;
    let mut resources = Vec::new();
    let dcst_path = if is_xml {
        input.to_path_buf()
    } else {
        let tmp = vf_dir.join(format!("{prefix}_{}.xml", uuid::Uuid::new_v4()));
        match crate::subtitle::srt_to_shifted_dcst(input, 0, lang, fps, opts, &tmp) {
            Ok(r) => resources = r,
            Err(e) => {
                tracing::error!("{prefix} conversion failed: {e}");
                return None;
            }
        }
        temp_dcst = Some(tmp.clone());
        tmp
    };

    let id = uuid::Uuid::new_v4();
    let filename = format!("{prefix}_{id}.mxf");
    let key = mint_key(key_type, &id.to_string()).ok()?;
    let track = crate::mxf_wrap::wrap_timed_text_resources(
        &dcst_path,
        &resources,
        &vf_dir.join(&filename),
        fps,
        Some(*id.as_bytes()),
        None,
        key.as_ref().map(crate::reel::mxf_enc),
    );
    if let Some(tmp) = temp_dcst {
        let _ = std::fs::remove_file(tmp);
    }
    // the staged font now lives inside the MXF; anything the caller supplied
    // stays where it is
    for (resource, _) in &resources {
        if resource.starts_with(vf_dir) {
            let _ = std::fs::remove_file(resource);
        }
    }
    let track = track?;
    let path = vf_dir.join(&filename);
    let hash = match crate::hash::hash_file(&path) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash {}: {e}", path.display());
            return None;
        }
    };
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let duration = if track.duration > 0 {
        track.duration
    } else {
        fallback_duration
    };
    Some(NewAsset {
        id: track.uuid,
        filename,
        hash,
        size,
        duration,
        key: key.map(|k| k.info),
    })
}

/// Parse a CPL EditRate string like "24 1" into (num, den), defaulting to 24/1.
fn parse_edit_rate(s: &str) -> (u32, u32) {
    let mut it = s.split_whitespace();
    let num = it.next().and_then(|v| v.parse().ok()).unwrap_or(24);
    let den = it.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    (num, den)
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Picture dimensions from the OV CPL's ScreenAspectRatio. SMPTE carries a
/// "w h" pair; Interop carries a decimal, from which we recover w at 1080p.
fn parse_screen_aspect(cpl: &str) -> (u32, u32) {
    let inner = cpl
        .split_once("<ScreenAspectRatio>")
        .and_then(|(_, r)| r.split_once("</ScreenAspectRatio>"))
        .map(|(v, _)| v.trim());
    match inner {
        Some(v) if v.contains(char::is_whitespace) => {
            let mut it = v.split_whitespace();
            let w = it.next().and_then(|x| x.parse().ok()).unwrap_or(2048);
            let h = it.next().and_then(|x| x.parse().ok()).unwrap_or(1080);
            (w, h)
        }
        Some(v) => match v.parse::<f64>() {
            Ok(r) if r > 0.0 => ((r * 1080.0).round() as u32, 1080),
            _ => (2048, 1080),
        },
        None => (2048, 1080),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIC_ID: &str = "11111111-1111-1111-1111-111111111111";
    const SND_ID: &str = "22222222-2222-2222-2222-222222222222";
    const CPL_ID: &str = "33333333-3333-3333-3333-333333333333";
    const PKL_ID: &str = "44444444-4444-4444-4444-444444444444";

    /// Write a minimal SMPTE OV DCP (ASSETMAP + CPL + PKL) with one reel.
    fn write_ov(dir: &Path) {
        let cpl = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CompositionPlaylist xmlns="http://www.smpte-ra.org/schemas/429-7/2006/CPL">
  <Id>urn:uuid:{CPL_ID}</Id>
  <ContentTitleText>OV Movie</ContentTitleText>
  <IssueDate>2026-01-01T00:00:00+00:00</IssueDate>
  <ContentKind>feature</ContentKind>
  <ReelList>
    <Reel>
      <Id>urn:uuid:55555555-5555-5555-5555-555555555555</Id>
      <AssetList>
        <MainPicture>
          <Id>urn:uuid:{PIC_ID}</Id>
          <EditRate>24 1</EditRate>
          <IntrinsicDuration>48</IntrinsicDuration>
          <Duration>48</Duration>
        </MainPicture>
        <MainSound>
          <Id>urn:uuid:{SND_ID}</Id>
          <EditRate>24 1</EditRate>
          <IntrinsicDuration>48</IntrinsicDuration>
          <Duration>48</Duration>
        </MainSound>
      </AssetList>
    </Reel>
  </ReelList>
</CompositionPlaylist>
"#
        );
        std::fs::write(dir.join(format!("CPL_{CPL_ID}.xml")), cpl).unwrap();
        std::fs::write(
            dir.join(format!("PKL_{PKL_ID}.xml")),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<PackingList xmlns="http://www.smpte-ra.org/schemas/429-8/2007/PKL">
  <Id>urn:uuid:{PKL_ID}</Id>
</PackingList>
"#
            ),
        )
        .unwrap();
        let am = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<AssetMap xmlns="http://www.smpte-ra.org/schemas/429-9/2007/AM">
  <Id>urn:uuid:66666666-6666-6666-6666-666666666666</Id>
  <AssetList>
    <Asset>
      <Id>urn:uuid:{PKL_ID}</Id>
      <PackingList>true</PackingList>
      <ChunkList><Chunk><Path>PKL_{PKL_ID}.xml</Path></Chunk></ChunkList>
    </Asset>
    <Asset>
      <Id>urn:uuid:{CPL_ID}</Id>
      <ChunkList><Chunk><Path>CPL_{CPL_ID}.xml</Path></Chunk></ChunkList>
    </Asset>
    <Asset>
      <Id>urn:uuid:{PIC_ID}</Id>
      <ChunkList><Chunk><Path>picture.mxf</Path></Chunk></ChunkList>
    </Asset>
    <Asset>
      <Id>urn:uuid:{SND_ID}</Id>
      <ChunkList><Chunk><Path>sound.mxf</Path></Chunk></ChunkList>
    </Asset>
  </AssetList>
</AssetMap>
"#
        );
        std::fs::write(dir.join("ASSETMAP.xml"), am).unwrap();
    }

    /// Wrap a second of stereo silence into a real sound MXF carrying `asset_id`.
    fn write_sound_mxf(dir: &Path, asset_id: uuid::Uuid) -> PathBuf {
        // the 48 frame reel `write_ov` declares, at 24 fps
        const REEL_SECONDS: u64 = 2;
        let sample_rate = 48_000u32;
        let channels = 2u16;
        let bits = 24u16;
        let block_align = (bits / 8) * channels;
        let data_len = REEL_SECONDS * sample_rate as u64 * block_align as u64;
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        w.extend_from_slice(b"WAVE");
        w.extend_from_slice(b"fmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&channels.to_le_bytes());
        w.extend_from_slice(&sample_rate.to_le_bytes());
        w.extend_from_slice(&(sample_rate * block_align as u32).to_le_bytes());
        w.extend_from_slice(&block_align.to_le_bytes());
        w.extend_from_slice(&bits.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(data_len as u32).to_le_bytes());
        w.resize(w.len() + data_len as usize, 0);
        let wav = dir.join("replacement.wav");
        std::fs::write(&wav, &w).unwrap();

        let mxf = dir.join("new_sound.mxf");
        crate::mxf_wrap::wrap_mxf_files(
            vec![wav],
            &mxf,
            crate::mxf_wrap::MxfType::PcmAudio,
            24,
            None,
            None,
            Some(*asset_id.as_bytes()),
        )
        .expect("wrap replacement sound MXF");
        mxf
    }

    /// Replacing a reel's sound must put the new MXF's real (registered) id in
    /// the CPL AND declare it in PKL + ASSETMAP, while unchanged reels stay refs.
    #[test]
    fn replaced_reel_registers_new_asset_everywhere() {
        let tmp = tempfile::tempdir().unwrap();
        let ov = tmp.path().join("ov");
        let vf = tmp.path().join("vf");
        std::fs::create_dir_all(&ov).unwrap();
        write_ov(&ov);

        // A pre-wrapped replacement ships under the id it already carries, so the
        // fixture has to be a real MXF with a known AssetUUID.
        let embedded_id = uuid::Uuid::new_v4();
        let new_snd = write_sound_mxf(tmp.path(), embedded_id);

        let config = VfConfig {
            ov_dir: ov.clone(),
            vf_dir: vf.clone(),
            title: String::new(),
            subtitle_language: String::new(),
            subtitle_opts: crate::subtitle::SubtitleOptions::default(),
            signer: None,
            replacement_reels: vec![ReplacementReel {
                reel_number: 1,
                picture: None,
                sound: Some(new_snd),
                subtitle: None,
                ccap: None,
            }],
        };
        assert_eq!(create_vf(&config), 0);
        let vf = vf_package_dir(&config).unwrap();

        // The new sound MXF ships in the VF; its id is embedded in its filename.
        let mxf = std::fs::read_dir(&vf)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with("sound_") && n.ends_with(".mxf"))
            .expect("new sound MXF present in VF");
        let new_id = mxf
            .trim_start_matches("sound_")
            .trim_end_matches(".mxf")
            .to_string();
        assert_eq!(
            new_id,
            embedded_id.to_string(),
            "the shipped file must be named with the id the MXF itself carries"
        );
        assert!(vf.join(&mxf).exists());

        let cpl_name = std::fs::read_dir(&vf)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with("CPL_"))
            .unwrap();
        let cpl = std::fs::read_to_string(vf.join(&cpl_name)).unwrap();
        let am = std::fs::read_to_string(vf.join("ASSETMAP.xml")).unwrap();
        let pkl_name = std::fs::read_dir(&vf)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.starts_with("PKL_"))
            .unwrap();
        let pkl = std::fs::read_to_string(vf.join(&pkl_name)).unwrap();

        // Replaced sound: real id in CPL, declared in PKL and ASSETMAP.
        assert!(
            cpl.contains(&format!("urn:uuid:{new_id}")),
            "CPL must reference the new sound's real id"
        );
        assert!(pkl.contains(&new_id), "PKL must declare the new sound");
        assert!(am.contains(&new_id), "ASSETMAP must declare the new sound");

        // Unchanged picture: referenced from the OV, not shipped/declared here.
        assert!(cpl.contains(PIC_ID), "CPL must still reference OV picture");
        assert!(
            !am.contains(PIC_ID),
            "ASSETMAP must not list the unchanged OV picture"
        );
        // Old OV sound id must be gone from the CPL (it was replaced).
        assert!(
            !cpl.contains(SND_ID),
            "replaced OV sound id must not remain in CPL"
        );
        assert_eq!(create_vf(&config), -1, "a second VF under the same title");
        assert_eq!(
            std::fs::read_to_string(vf.join(&cpl_name)).unwrap(),
            cpl,
            "the refused VF leaves the first one alone"
        );
    }

    #[test]
    fn fails_when_nothing_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let ov = tmp.path().join("ov");
        std::fs::create_dir_all(&ov).unwrap();
        write_ov(&ov);
        let config = VfConfig {
            ov_dir: ov,
            vf_dir: tmp.path().join("vf"),
            title: String::new(),
            subtitle_language: String::new(),
            subtitle_opts: crate::subtitle::SubtitleOptions::default(),
            signer: None,
            replacement_reels: vec![],
        };
        assert_eq!(create_vf(&config), -1);
    }

    #[test]
    fn fails_on_unknown_reel() {
        let tmp = tempfile::tempdir().unwrap();
        let ov = tmp.path().join("ov");
        std::fs::create_dir_all(&ov).unwrap();
        write_ov(&ov);
        let snd = tmp.path().join("s.mxf");
        std::fs::write(&snd, b"x").unwrap();
        let config = VfConfig {
            ov_dir: ov,
            vf_dir: tmp.path().join("vf"),
            title: String::new(),
            subtitle_language: String::new(),
            subtitle_opts: crate::subtitle::SubtitleOptions::default(),
            signer: None,
            replacement_reels: vec![ReplacementReel {
                reel_number: 9,
                picture: None,
                sound: Some(snd),
                subtitle: None,
                ccap: None,
            }],
        };
        assert_eq!(create_vf(&config), -1);
    }
}
