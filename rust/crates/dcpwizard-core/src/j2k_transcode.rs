use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use asdcplib::jp2k::MxfReader;
use postkit::grok_encoder::RawFrame;

/// 16 MB covers a single 4K J2K frame.
const MAX_FRAME_BUF: usize = 16 * 1024 * 1024;

/// Bits in a byte, turning summed codestream lengths into a bandwidth.
const BITS_PER_BYTE: u64 = 8;

/// Bits in a megabit, the unit a picture bandwidth is quoted in.
const BITS_PER_MEGABIT: u64 = 1_000_000;

/// The CPL element a stereoscopic reel declares its picture as (ST 429-10).
const STEREOSCOPIC_PICTURE_ELEMENT: &str = "MainStereoscopicPicture";

// the re-encode reports 0 to this, the package files the rest
const PACKAGING_PERCENT: u32 = 95;
const DONE_PERCENT: u32 = 100;

/// Re-encode an existing DCP's picture essence at a different bandwidth (and,
/// optionally, into another container), copying audio and subtitle tracks unchanged. With
/// `watermark` set the same pass marks the picture, which is what the
/// `watermark` command is.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcpTranscodeConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    /// target picture bandwidth in Mbit/s. `None` re-encodes each reel at the
    /// source picture's own average bandwidth, which is what marking a DCP
    /// asks for: the mark is the only thing meant to change.
    pub target_bitrate_mbps: Option<u32>,
    /// container raster the picture is fitted into with its aspect kept.
    /// `None` keeps the source raster untouched.
    pub container: Option<(u32, u32)>,
    /// KDM XML for an encrypted source (with `recipient_key`). Each source frame
    /// is decrypted and decoded in memory; the re-encoded output is cleartext.
    pub kdm: Option<PathBuf>,
    /// Recipient RSA private key (PEM) matching `kdm`.
    pub recipient_key: Option<PathBuf>,
    /// dcpwizard KEYS.json, an alternative key source to `kdm`.
    pub keys: Option<PathBuf>,
    /// A visible mark burnt into every re-encoded frame. Not serialised: it
    /// carries a live font database, so a stored job names the text and
    /// rebuilds it.
    #[serde(skip)]
    pub watermark: Option<Arc<postkit::subtitle_raster::SubtitleBurn>>,
    // the running process's setting, never read from a job
    #[serde(skip)]
    pub encode_threads: u32,
}

/// One MXF that ships in the output DCP (declared in CPL/PKL/ASSETMAP).
struct ShippedAsset {
    id: String,
    filename: String,
    hash: String,
    size: u64,
}

fn non_empty(hash: &str) -> Option<String> {
    (!hash.is_empty()).then(|| hash.to_string())
}

/// Result of re-encoding one reel's picture track.
struct NewPicture {
    id: String,
    filename: String,
    hash: String,
    size: u64,
    duration: u64,
    width: u32,
    height: u32,
    edit_rate_num: u32,
    edit_rate_den: u32,
}

/// Refuse a transcode that names no bandwidth, no mark and no container other
/// than the source's own raster: the picture would only lose quality.
pub fn check_picture_changes(config: &DcpTranscodeConfig) -> Result<(), String> {
    if config.target_bitrate_mbps.is_some() || config.watermark.is_some() {
        return Ok(());
    }
    if let Some(container) = config.container
        && source_picture_rasters(&config.input_dir)?
            .iter()
            .any(|raster| *raster != container)
    {
        return Ok(());
    }
    Err(format!(
        "nothing would change in {}: without --video-bit-rate or a container whose raster \
         differs from the source picture's, the picture would be re-encoded as it is and \
         only lose quality",
        config.input_dir.display()
    ))
}

/// The stored raster of every reel's picture, read from the MXF descriptors.
fn source_picture_rasters(input_dir: &Path) -> Result<Vec<(u32, u32)>, String> {
    let cpls = crate::multi_cpl::list_cpls(input_dir);
    let cpl = cpls
        .first()
        .ok_or_else(|| format!("No CPL found in {}", input_dir.display()))?;
    crate::multi_cpl::get_timeline(&input_dir.join(&cpl.file_path))
        .iter()
        .map(|entry| {
            let mut reader = MxfReader::new();
            reader
                .open_read(&entry.picture_file)
                .map_err(|e| format!("Failed to open picture MXF {}: {e}", entry.picture_file))?;
            let descriptor = reader
                .picture_descriptor()
                .map_err(|e| format!("Failed to read picture descriptor: {e}"))?;
            Ok((descriptor.stored_width, descriptor.stored_height))
        })
        .collect()
}

/// Transcode an existing DCP: re-encode every reel's picture essence to the
/// target bandwidth, copy audio/subtitle tracks verbatim, and emit a fresh
/// CPL/PKL/ASSETMAP. Fails loud on encrypted input.
pub fn transcode_dcp(config: &DcpTranscodeConfig) -> i32 {
    match transcode_dcp_with_progress(config, &crate::dcp::NoProgress) {
        Ok(()) => 0,
        Err(message) => {
            tracing::error!("{message}");
            -1
        }
    }
}

pub fn transcode_dcp_with_progress(
    config: &DcpTranscodeConfig,
    progress: &dyn crate::dcp::ProgressSink,
) -> Result<(), String> {
    if config.target_bitrate_mbps == Some(0) {
        return Err("--video-bit-rate must be > 0".into());
    }
    if !config.input_dir.exists() {
        return Err(format!(
            "Input DCP not found: {}",
            config.input_dir.display()
        ));
    }
    if config.input_dir == config.output_dir {
        return Err("output must differ from input".into());
    }

    let cpls = crate::multi_cpl::list_cpls(&config.input_dir);
    let Some(cpl) = cpls.first() else {
        return Err(format!("No CPL found in {}", config.input_dir.display()));
    };
    let cpl_path = config.input_dir.join(&cpl.file_path);
    let cpl_content = std::fs::read_to_string(&cpl_path).unwrap_or_default();
    let timeline = crate::multi_cpl::get_timeline(&cpl_path);
    if timeline.is_empty() {
        return Err("CPL has no reels".into());
    }

    // encrypted input needs key material: with a KDM+recipient key or KEYS.json
    // each source frame is decrypted in memory before decode; without it we
    // cannot re-encode what we cannot decode, so fail loud.
    let key_source = postkit::content_keys::ContentKeys::from_options(
        config.kdm.as_deref(),
        config.recipient_key.as_deref(),
        config.keys.as_deref(),
    )?;
    if cpl_content.contains("<KeyId>") && key_source.is_none() {
        return Err(
            "input DCP is encrypted; supply --kdm + --recipient-key or --keys to transcode it"
                .into(),
        );
    }

    // both eyes share one picture track, and this path re-encodes a track as a
    // single eye, so a stereoscopic source would come back with one eye
    if cpl_content.contains(STEREOSCOPIC_PICTURE_ELEMENT) {
        return Err(format!(
            "{} is stereoscopic: re-encoding its picture here would keep one eye of the two, \
             so it is refused",
            config.input_dir.display()
        ));
    }

    let standard = if cpl_content.contains("digicine.com") {
        crate::Standard::Interop
    } else {
        crate::Standard::Smpte
    };
    let source_active_area = crate::cpl::active_area_from_cpl(&cpl_content);

    std::fs::create_dir_all(&config.output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    let mut cpl_reels: Vec<crate::cpl::CplReel> = Vec::new();
    let mut shipped: Vec<ShippedAsset> = Vec::new();
    // re-encoding keeps the frame count, so marker offsets carry over unchanged
    let source_markers = crate::markers::markers_from_cpl(&cpl_content);

    let reel_frame_counts = timeline
        .iter()
        .map(|entry| {
            if entry.picture_file.is_empty() || !Path::new(&entry.picture_file).exists() {
                return Err(format!("reel {} picture MXF not found", entry.reel_number));
            }
            picture_frame_count(Path::new(&entry.picture_file))
        })
        .collect::<Result<Vec<u64>, String>>()?;
    let total_frames: u64 = reel_frame_counts.iter().sum();
    let mut frames_before = 0u64;

    for (reel_index, entry) in timeline.iter().enumerate() {
        if progress.cancelled() {
            return Err(crate::dcp::CANCELLED_MESSAGE.into());
        }
        progress.stage(
            percent_of(frames_before, total_frames),
            &format!(
                "re-encoding the picture of reel {} of {}",
                reel_index + 1,
                timeline.len()
            ),
        );
        let src_pic = PathBuf::from(&entry.picture_file);
        let pic = transcode_picture(
            &src_pic,
            config,
            key_source.as_ref(),
            source_active_area,
            standard,
            &FrameProgress {
                sink: progress,
                frames_before,
                total_frames,
            },
        )?;
        frames_before += reel_frame_counts[reel_index];

        // every non-picture track: cleartext copies verbatim (asset id preserved);
        // with a key source, an encrypted one is decrypted and rewrapped so the
        // cleartext output stays coherent.
        let fps_snd = (pic.edit_rate_num as f64 / pic.edit_rate_den as f64).round() as u32;
        let sound = sound_track(
            &entry.sound_file,
            &entry.sound_asset_id,
            key_source.as_ref(),
            fps_snd,
            &config.output_dir,
        )?;
        let subtitle = timed_text_track(
            &entry.subtitle_file,
            &entry.subtitle_asset_id,
            "subtitle",
            key_source.as_ref(),
            &config.output_dir,
        )?;
        let ccap = timed_text_track(
            &entry.ccap_file,
            &entry.ccap_asset_id,
            "ccap",
            key_source.as_ref(),
            &config.output_dir,
        )?;
        let aux = aux_data_track(entry, key_source.as_ref(), &config.output_dir)?;

        let subtitle_lang = if entry.subtitle_language.is_empty() {
            None
        } else {
            Some(entry.subtitle_language.clone())
        };
        let ccap_lang = if entry.ccap_language.is_empty() {
            None
        } else {
            Some(entry.ccap_language.clone())
        };

        cpl_reels.push(crate::cpl::CplReel {
            reel_id: uuid::Uuid::new_v4().to_string(),
            picture_id: pic.id.clone(),
            picture_width: pic.width,
            picture_height: pic.height,
            picture_edit_rate_num: pic.edit_rate_num,
            picture_edit_rate_den: pic.edit_rate_den,
            picture_duration: pic.duration,
            picture_entry_point: 0,
            picture_key_id: None,
            picture_hash: non_empty(&pic.hash),
            sound_id: sound.as_ref().map(|s| s.id.clone()),
            sound_edit_rate_num: pic.edit_rate_num,
            sound_edit_rate_den: pic.edit_rate_den,
            sound_duration: if sound.is_some() { pic.duration } else { 0 },
            sound_entry_point: 0,
            sound_key_id: None,
            sound_hash: sound.as_ref().and_then(|s| non_empty(&s.hash)),
            subtitle_id: subtitle.as_ref().map(|(s, _)| s.id.clone()),
            subtitle_edit_rate_num: pic.edit_rate_num,
            subtitle_edit_rate_den: pic.edit_rate_den,
            subtitle_duration: subtitle.as_ref().map(|(_, d)| *d).unwrap_or(0),
            subtitle_entry_point: 0,
            subtitle_language: subtitle_lang,
            subtitle_hash: subtitle.as_ref().and_then(|(s, _)| non_empty(&s.hash)),
            ccap_id: ccap.as_ref().map(|(s, _)| s.id.clone()),
            ccap_edit_rate_num: pic.edit_rate_num,
            ccap_edit_rate_den: pic.edit_rate_den,
            ccap_duration: ccap.as_ref().map(|(_, d)| *d).unwrap_or(0),
            ccap_entry_point: 0,
            ccap_language: ccap_lang,
            ccap_hash: ccap.as_ref().and_then(|(s, _)| non_empty(&s.hash)),
            stereoscopic: false,
            aux_data: aux.as_ref().map(|(_, a)| a.clone()),
            markers: source_markers.get(reel_index).cloned().unwrap_or_default(),
            ..Default::default()
        });

        shipped.push(ShippedAsset {
            id: pic.id,
            filename: pic.filename,
            hash: pic.hash,
            size: pic.size,
        });
        if let Some(s) = sound {
            shipped.push(s);
        }
        if let Some((s, _)) = subtitle {
            shipped.push(s);
        }
        if let Some((s, _)) = ccap {
            shipped.push(s);
        }
        if let Some((s, _)) = aux {
            shipped.push(s);
        }
    }

    if progress.cancelled() {
        return Err(crate::dcp::CANCELLED_MESSAGE.into());
    }
    progress.stage(PACKAGING_PERCENT, "writing CPL/PKL/ASSETMAP");
    // ── CPL ────────────────────────────────────────────────────────────
    let cpl_uuid = uuid::Uuid::new_v4().to_string();
    let out_cpl_path = config.output_dir.join(format!("CPL_{cpl_uuid}.xml"));
    let content_kind = if cpl.content_kind.is_empty() {
        "feature".to_string()
    } else {
        cpl.content_kind.clone()
    };
    let cpl_config = crate::cpl::CplConfig {
        title: cpl.content_title.clone(),
        content_kind,
        rating: String::new(),
        reels: cpl_reels,
        standard,
        // only the picture is re-encoded, so the source's sound layout still holds
        main_sound: crate::cpl::main_sound_from_cpl(&cpl_content),
        sign_language: None,
        ..Default::default()
    };
    if crate::cpl::generate_cpl(&cpl_config, &cpl_uuid, &out_cpl_path) != 0 {
        return Err("Failed to generate CPL".into());
    }

    // ── PKL ────────────────────────────────────────────────────────────
    let pkl_uuid = uuid::Uuid::new_v4().to_string();
    let cpl_hash = crate::hash::hash_file(&out_cpl_path).unwrap_or_default();
    let cpl_size = std::fs::metadata(&out_cpl_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let mut pkl_entries = vec![crate::pkl::PklEntry {
        id: cpl_uuid.clone(),
        asset_type: "text/xml".into(),
        file: out_cpl_path.clone(),
        hash: cpl_hash,
        size: cpl_size,
    }];
    for a in &shipped {
        pkl_entries.push(crate::pkl::PklEntry {
            id: a.id.clone(),
            asset_type: "application/mxf".into(),
            file: config.output_dir.join(&a.filename),
            hash: a.hash.clone(),
            size: a.size,
        });
    }
    let pkl_path = config.output_dir.join(format!("PKL_{pkl_uuid}.xml"));
    if crate::pkl::generate_pkl(
        &pkl_entries,
        &pkl_uuid,
        standard,
        Some(&cpl_config.title),
        &pkl_path,
    ) != 0
    {
        return Err("Failed to generate PKL".into());
    }

    // ── ASSETMAP ───────────────────────────────────────────────────────
    let mut am_entries = vec![
        crate::assetmap::AssetMapEntry {
            id: pkl_uuid,
            path: file_name(&pkl_path),
            packing_list: true,
        },
        crate::assetmap::AssetMapEntry {
            id: cpl_uuid,
            path: file_name(&out_cpl_path),
            packing_list: false,
        },
    ];
    for a in &shipped {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: a.id.clone(),
            path: a.filename.clone(),
            packing_list: false,
        });
    }
    if crate::assetmap::generate_assetmap(&am_entries, &config.output_dir, standard, None) != 0 {
        return Err("Failed to generate ASSETMAP".into());
    }

    match config.target_bitrate_mbps {
        Some(mbps) => tracing::info!(
            "Transcoded DCP to {} ({} reel(s) re-encoded at {mbps} Mbps)",
            config.output_dir.display(),
            timeline.len(),
        ),
        None => tracing::info!(
            "Transcoded DCP to {} ({} reel(s) re-encoded at the source bandwidth)",
            config.output_dir.display(),
            timeline.len(),
        ),
    }
    progress.stage(DONE_PERCENT, "done");
    Ok(())
}

// how far through the re-encode a reel starts, from the frames already encoded
fn percent_of(frames_done: u64, total_frames: u64) -> u32 {
    (frames_done * u64::from(PACKAGING_PERCENT) / total_frames.max(1)) as u32
}

fn picture_frame_count(picture_file: &Path) -> Result<u64, String> {
    let mut reader = MxfReader::new();
    reader
        .open_read(&picture_file.to_string_lossy())
        .map_err(|e| format!("Failed to open picture MXF {}: {e}", picture_file.display()))?;
    let descriptor = reader
        .picture_descriptor()
        .map_err(|e| format!("Failed to read picture descriptor: {e}"))?;
    Ok(u64::from(descriptor.container_duration))
}

struct FrameProgress<'a> {
    sink: &'a dyn crate::dcp::ProgressSink,
    frames_before: u64,
    total_frames: u64,
}

/// Decode one picture MXF in memory and re-encode it at the bandwidth's bytes a frame.
fn transcode_picture(
    src_mxf: &Path,
    config: &DcpTranscodeConfig,
    key_source: Option<&postkit::content_keys::ContentKeys>,
    source_active_area: Option<(u32, u32)>,
    standard: crate::Standard,
    progress: &FrameProgress,
) -> Result<NewPicture, String> {
    let mut reader = MxfReader::new();
    reader
        .open_read(&src_mxf.to_string_lossy())
        .map_err(|e| format!("Failed to open picture MXF {}: {e}", src_mxf.display()))?;
    let desc = reader
        .picture_descriptor()
        .map_err(|e| format!("Failed to read picture descriptor: {e}"))?;
    // encrypted source: build the AES/HMAC contexts from the key source, keyed by
    // this MXF's own KeyId, so every read_frame below decrypts in memory.
    let info = reader
        .writer_info()
        .map_err(|e| format!("Failed to read picture writer info: {e}"))?;
    let key_source = key_source.filter(|_| info.encrypted_essence);
    if info.encrypted_essence {
        let Some(ks) = key_source else {
            return Err(format!(
                "picture MXF {} is encrypted and no key source was given",
                src_mxf.display()
            ));
        };
        ks.decrypt_and_hmac_contexts(&info, "picture")?;
    }
    let frame_count = desc.container_duration;
    if frame_count == 0 {
        return Err(format!("picture MXF {} has no frames", src_mxf.display()));
    }
    let src_w = desc.stored_width;
    let src_h = desc.stored_height;
    let edit_num = desc.edit_rate.numerator.max(1) as u32;
    let edit_den = desc.edit_rate.denominator.max(1) as u32;
    let fps = (edit_num as f64 / edit_den as f64).round() as u32;

    let (out_w, out_h) = config.container.unwrap_or((src_w, src_h));
    let fit_plan = config
        .container
        .map(|container| {
            container_fit(
                src_mxf,
                (src_w, src_h),
                source_active_area,
                container,
                fps,
                standard,
            )
        })
        .transpose()?
        .filter(|plan| !plan.is_identity());
    if let Some(plan) = &fit_plan {
        tracing::info!("{}: {}", src_mxf.display(), plan.describe());
    }
    let fit_filters = fit_plan.map(|plan| plan.filters);

    let bitrate_mbps = match config.target_bitrate_mbps {
        Some(mbps) => mbps,
        None => {
            let measured = source_bandwidth_mbps(src_mxf, key_source, &info, frame_count, fps)?;
            tracing::info!(
                "{} averages {measured} Mbit/s, which the re-encode targets",
                src_mxf.display()
            );
            measured
        }
    };
    let target_codestream_bytes =
        crate::encode::video_codestream_byte_cap(fps, bitrate_mbps, false);

    let work = config
        .output_dir
        .join(format!(".transcode_{}", uuid::Uuid::new_v4()));
    let j2k_dir = work.join("j2k");
    std::fs::create_dir_all(&j2k_dir)
        .map_err(|e| format!("Failed to create transcode work dir: {e}"))?;

    let fit_filters = fit_filters.as_deref();
    let writer_info = &info;
    let open_loader = || -> Result<postkit::encode::FrameLoader<'_>, String> {
        let mut reader = MxfReader::new();
        reader
            .open_read(&src_mxf.to_string_lossy())
            .map_err(|e| format!("Failed to open picture MXF {}: {e}", src_mxf.display()))?;
        let mut crypto = match key_source {
            Some(ks) => Some(ks.decrypt_and_hmac_contexts(writer_info, "picture")?),
            None => None,
        };
        let mut buf = vec![0u8; MAX_FRAME_BUF];
        let mut fit = fit_filters.map(|filters| FrameFit {
            filters,
            graph: None,
        });
        Ok(Box::new(move |frame_index: u64| {
            decode_source_frame(
                &mut reader,
                crypto.as_mut(),
                frame_index as u32,
                &mut buf,
                fit.as_mut(),
            )
        }))
    };
    // AlreadyPq is the source colour that compresses untransformed
    let options = postkit::encode::StreamEncodeOptions {
        output_dir: j2k_dir.clone(),
        compression_ratio: crate::encode::DEFAULT_COMPRESSION_RATIO,
        target_codestream_bytes: Some(target_codestream_bytes),
        fps: postkit::encode::FrameRate::whole(fps),
        source_colour: postkit::encode::SourceColour::AlreadyPq,
        watermark: config.watermark.clone(),
        encode_threads: config.encode_threads,
        ..Default::default()
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let result = postkit::encode::encode_loaded_frames(
        u64::from(frame_count),
        open_loader,
        &options,
        &cancel,
        &Arc::new(AtomicBool::new(false)),
        None,
        |encoded| {
            progress.sink.frames(
                progress.frames_before + encoded.frame,
                progress.total_frames,
            );
            if progress.sink.cancelled() {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    );
    if cancel.load(Ordering::Relaxed) {
        let _ = std::fs::remove_dir_all(&work);
        return Err(crate::dcp::CANCELLED_MESSAGE.into());
    }
    if !result.success {
        let _ = std::fs::remove_dir_all(&work);
        return Err(format!("re-encode failed: {}", result.error));
    }

    let id = uuid::Uuid::new_v4();
    let filename = format!("picture_{id}.mxf");
    let out_mxf = config.output_dir.join(&filename);
    let track = crate::mxf_wrap::wrap_mxf_files(
        sorted_files(&j2k_dir),
        &out_mxf,
        crate::mxf_wrap::MxfType::J2kPicture,
        fps,
        None,
        None,
        Some(*id.as_bytes()),
    );
    let _ = std::fs::remove_dir_all(&work);
    let track = track.ok_or_else(|| {
        format!(
            "Failed to wrap the re-encoded picture into {}",
            out_mxf.display()
        )
    })?;

    let hash = crate::hash::hash_file(&out_mxf)?;
    let size = std::fs::metadata(&out_mxf).map(|m| m.len()).unwrap_or(0);
    Ok(NewPicture {
        id: track.uuid,
        filename,
        hash,
        size,
        duration: if track.duration > 0 {
            track.duration
        } else {
            frame_count as u64
        },
        width: out_w,
        height: out_h,
        edit_rate_num: edit_num,
        edit_rate_den: edit_den,
    })
}

/// Crop the source to the active area its CPL declares, centred in the stored
/// raster, then fit that onto the container the way `create` fits a source.
fn container_fit(
    src_mxf: &Path,
    source_raster: (u32, u32),
    source_active_area: Option<(u32, u32)>,
    container: (u32, u32),
    fps: u32,
    standard: crate::Standard,
) -> Result<postkit::picture_processing::PicturePlan, String> {
    let is_4k = crate::Resolution::for_raster(container.0, container.1) == crate::Resolution::FourK;
    crate::hfr::validate_fps_resolution(fps, is_4k, standard == crate::Standard::Smpte)?;
    let (stored_width, stored_height) = source_raster;
    let (active_width, active_height) = source_active_area.unwrap_or(source_raster);
    let unused_width = stored_width.saturating_sub(active_width);
    let unused_height = stored_height.saturating_sub(active_height);
    let options = crate::source_picture::SourcePictureOptions {
        crop: postkit::picture_processing::Crop {
            left: unused_width / 2,
            right: unused_width - unused_width / 2,
            top: unused_height / 2,
            bottom: unused_height - unused_height / 2,
        },
        ..Default::default()
    };
    let geometry = crate::source_picture::EncodeGeometry {
        forced_raster: Some(container),
        container: Some(container),
    };
    let resolved = crate::source_picture::resolve_picture(
        &options,
        src_mxf,
        stored_width,
        stored_height,
        &geometry,
        false,
    )?;
    Ok(resolved.plan)
}

/// The picture's own average bandwidth in Mbit/s, rounded up: every frame's
/// codestream length summed, at the picture's rate. The lengths come from the
/// reads themselves, so no frame is decoded.
fn source_bandwidth_mbps(
    src_mxf: &Path,
    key_source: Option<&postkit::content_keys::ContentKeys>,
    writer_info: &asdcplib::WriterInfo,
    frame_count: u32,
    fps: u32,
) -> Result<u32, String> {
    let mut reader = MxfReader::new();
    reader
        .open_read(&src_mxf.to_string_lossy())
        .map_err(|e| format!("Failed to open picture MXF {}: {e}", src_mxf.display()))?;
    let mut crypto = match key_source {
        Some(ks) => Some(ks.decrypt_and_hmac_contexts(writer_info, "picture")?),
        None => None,
    };
    let mut buf = vec![0u8; MAX_FRAME_BUF];
    let mut total_bytes = 0u64;
    for index in 0..frame_count {
        let (dec, hmac) = match crypto.as_mut() {
            Some((d, h)) => (Some(d), Some(h)),
            None => (None, None),
        };
        total_bytes += reader
            .read_frame(index, &mut buf, dec, hmac)
            .map_err(|e| format!("cannot read frame {index} of {}: {e}", src_mxf.display()))?
            as u64;
    }
    let bits_per_second = total_bytes * BITS_PER_BYTE * u64::from(fps) / u64::from(frame_count);
    Ok(bits_per_second.div_ceil(BITS_PER_MEGABIT) as u32)
}

/// Copy an essence MXF into the output DCP unchanged, keeping its asset id.
/// Returns `Ok(None)` when there is no such track.
fn copy_track(
    src_file: &str,
    asset_id: &str,
    prefix: &str,
    out_dir: &Path,
) -> Result<Option<ShippedAsset>, String> {
    if src_file.is_empty() || asset_id.is_empty() {
        return Ok(None);
    }
    let src = PathBuf::from(src_file);
    if !src.exists() {
        return Err(format!("{prefix} MXF not found: {src_file}"));
    }
    let filename = format!("{prefix}_{asset_id}.mxf");
    let dst = out_dir.join(&filename);
    std::fs::copy(&src, &dst).map_err(|e| format!("failed to copy {prefix} MXF: {e}"))?;
    let hash = crate::hash::hash_file(&dst)?;
    let size = std::fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
    Ok(Some(ShippedAsset {
        id: asset_id.to_string(),
        filename,
        hash,
        size,
    }))
}

/// Resolve the sound track for the output: without a key source, copy verbatim
/// (asset id preserved); with one, an encrypted sound is decrypted and rewrapped
/// as cleartext (via the shared decrypt path) so the output CPL stays coherent.
fn sound_track(
    src_file: &str,
    asset_id: &str,
    key_source: Option<&postkit::content_keys::ContentKeys>,
    fps: u32,
    out_dir: &Path,
) -> Result<Option<ShippedAsset>, String> {
    match key_source {
        Some(ks) => Ok(
            crate::decrypt::process_sound(src_file, asset_id, ks, fps, out_dir)?.map(from_decrypt),
        ),
        None => copy_track(src_file, asset_id, "sound", out_dir),
    }
}

/// Resolve a timed-text track (subtitle or closed caption) for the output, with
/// the frame count the essence declares: a cleartext track copies verbatim
/// (asset id preserved), an encrypted one is decrypted and rewrapped as
/// cleartext.
fn timed_text_track(
    src_file: &str,
    asset_id: &str,
    prefix: &str,
    key_source: Option<&postkit::content_keys::ContentKeys>,
    out_dir: &Path,
) -> Result<Option<(ShippedAsset, u64)>, String> {
    Ok(
        crate::decrypt::process_timed_text(src_file, asset_id, prefix, key_source, out_dir)?
            .map(|(asset, duration)| (from_decrypt(asset), duration)),
    )
}

/// Resolve the Atmos / DCData auxiliary track, decrypting it when the source is
/// encrypted. A cleartext copy still goes through the decrypt path, since the
/// rebuilt reel declares the track's edit rate and duration off its descriptor.
fn aux_data_track(
    entry: &crate::multi_cpl::TimelineEntry,
    key_source: Option<&postkit::content_keys::ContentKeys>,
    out_dir: &Path,
) -> Result<Option<(ShippedAsset, crate::cpl::AuxData)>, String> {
    Ok(crate::decrypt::process_aux_data(
        &entry.aux_data_file,
        &entry.aux_data_asset_id,
        &entry.aux_data_type,
        key_source,
        out_dir,
    )?
    .map(|(s, a)| (from_decrypt(s), a)))
}

/// Map a shared decrypt-path asset onto this module's ShippedAsset.
fn from_decrypt(s: crate::decrypt::ShippedAsset) -> ShippedAsset {
    ShippedAsset {
        id: s.id,
        filename: s.filename,
        hash: s.hash,
        size: s.size,
    }
}

// one per loader thread, the graph built from its first frame and kept for the rest
struct FrameFit<'a> {
    filters: &'a [String],
    graph: Option<postkit::planar_rgb_filter::PlanarRgbFilter>,
}

/// Read one source frame, decrypting it when the essence is encrypted, decode
/// it in memory, run the fit over it when there is one, and hand it to the
/// encoder as planar components.
fn decode_source_frame(
    reader: &mut MxfReader,
    crypto: Option<&mut (
        asdcplib::crypto::AesDecContext,
        asdcplib::crypto::HmacContext,
    )>,
    index: u32,
    buf: &mut [u8],
    fit: Option<&mut FrameFit>,
) -> Result<RawFrame, String> {
    let (dec, hmac) = match crypto {
        Some((d, h)) => (Some(d), Some(h)),
        None => (None, None),
    };
    let n = reader
        .read_frame(index, buf, dec, hmac)
        .map_err(|e| format!("Failed to read frame {index} (wrong key or MIC mismatch): {e}"))?;
    let decoded = postkit::grok_decoder::decode(buf[..n].to_vec(), 0)
        .map_err(|e| format!("cannot decode frame {index}: {e}"))?;
    let components: [Vec<i32>; 3] = decoded
        .components
        .try_into()
        .map_err(|_| format!("frame {index} does not carry three components"))?;
    let Some(fit) = fit else {
        return Ok(RawFrame::Planar {
            components,
            width: decoded.width,
            height: decoded.height,
            precision: decoded.precision,
            index: u64::from(index),
        });
    };
    let cannot_fit = |e: String| format!("cannot fit frame {index} into the container: {e}");
    let graph = match &mut fit.graph {
        Some(graph) => graph,
        None => fit.graph.insert(
            postkit::planar_rgb_filter::PlanarRgbFilter::new(
                decoded.width,
                decoded.height,
                decoded.precision,
                fit.filters,
            )
            .map_err(cannot_fit)?,
        ),
    };
    let (width, height, components) = graph.filter(&components).map_err(cannot_fit)?;
    Ok(RawFrame::Planar {
        components,
        width,
        height,
        precision: decoded.precision,
        index: u64::from(index),
    })
}

fn sorted_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    v.sort();
    v
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_track_absent_is_none() {
        let dir = tempfile::tempdir().unwrap();
        // no file and no id -> no track to ship
        assert!(copy_track("", "", "sound", dir.path()).unwrap().is_none());
        assert!(
            copy_track("", "some-id", "sound", dir.path())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn copy_track_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let r = copy_track("/nope/missing.mxf", "abc", "sound", dir.path());
        assert!(r.is_err());
    }

    #[test]
    fn copy_track_copies_and_keeps_id() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.mxf");
        std::fs::write(&src, b"essence bytes").unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let a = copy_track(&src.to_string_lossy(), "the-id", "sound", &out)
            .unwrap()
            .unwrap();
        assert_eq!(a.id, "the-id");
        assert_eq!(a.filename, "sound_the-id.mxf");
        assert!(out.join(&a.filename).exists());
        assert_eq!(a.size, 13);
    }

    const FLAT: (u32, u32) = (1998, 1080);
    const FOUR_K_FLAT: (u32, u32) = (3996, 2160);
    const HIGH_FRAME_RATE: u32 = 48;

    #[test]
    fn a_container_the_source_already_fills_runs_no_filters() {
        let plan = container_fit(
            Path::new("/source/never-read.mxf"),
            FLAT,
            Some(FLAT),
            FLAT,
            24,
            crate::Standard::Smpte,
        )
        .unwrap();
        assert!(plan.is_identity(), "{:?}", plan.filters);
    }

    #[test]
    fn a_four_k_container_at_a_high_frame_rate_is_refused() {
        let refused = container_fit(
            Path::new("/source/never-read.mxf"),
            FLAT,
            None,
            FOUR_K_FLAT,
            HIGH_FRAME_RATE,
            crate::Standard::Smpte,
        )
        .unwrap_err();
        assert!(refused.contains("requires a 2K container"), "{refused}");
    }

    #[test]
    fn transcode_rejects_zero_bitrate() {
        let dir = tempfile::tempdir().unwrap();
        let config = DcpTranscodeConfig {
            input_dir: dir.path().to_path_buf(),
            output_dir: dir.path().join("out"),
            target_bitrate_mbps: Some(0),
            ..Default::default()
        };
        assert_eq!(transcode_dcp(&config), -1);
    }
}
