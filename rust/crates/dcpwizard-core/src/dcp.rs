use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// DCP creation configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcpConfig {
    pub title: String,
    pub standard: crate::Standard,
    pub resolution: crate::Resolution,
    pub content_type: crate::ContentType,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub max_bitrate_mbps: u32,
    pub encrypt: bool,
    /// Where to write the content keys when `encrypt` is set. Required for
    /// encryption; keys are never written next to the DCP by default.
    pub key_out: Option<PathBuf>,
    pub stereo_3d: bool,
    /// Explicit CPL container dimensions (e.g. 2048x858 scope, 1998x1080 flat).
    /// Zero falls back to the `resolution` preset (full-container 2K/4K).
    pub container_width: u32,
    pub container_height: u32,
    pub output_dir: PathBuf,
    /// Directory containing J2K frames to wrap into picture MXF
    pub j2k_dir: Option<PathBuf>,
    /// Audio WAV file to wrap into sound MXF
    pub audio_path: Option<PathBuf>,
    /// Declared channel order for a six-channel input WAV. DCPwizard never
    /// guesses this order.
    pub audio_input_order: crate::mxf_wrap::AudioInputOrder,
    /// SRT subtitle file to convert, or supplied SMPTE timed-text XML to wrap as
    /// a subtitle track.
    pub subtitle_path: Option<PathBuf>,
    /// Subtitle language code (default "en").
    pub subtitle_language: String,
    /// Split the DCP into reels of at most this many minutes each. Zero (default)
    /// keeps the single-reel path.
    pub reel_length_minutes: u32,
    /// Right-eye J2K frames for a stereoscopic (ST 429-10) DCP; the main `j2k_dir`
    /// is the left eye. Present implies a 3D package.
    pub right_eye_dir: Option<PathBuf>,
    /// Dolby Atmos / DCData bitstream to wrap as a ST 429-18 auxiliary track.
    pub atmos_path: Option<PathBuf>,
    /// Sound channel index carrying the Hearing Impaired (HI) accessibility track.
    pub hi_channel: Option<u32>,
    /// Sound channel index carrying the Visually Impaired (VI-N) narration track.
    pub vi_channel: Option<u32>,
    /// Black-frame + silence padding prepended at the head of the program. A
    /// duration with a unit: frames (`48f`) or seconds (`2s`). Head padding
    /// shifts the program, so supplied SRT subtitles are re-timed by this offset.
    pub pad_head: Option<String>,
    /// Black-frame + silence padding appended at the tail of the program. Same
    /// syntax as `pad_head`.
    pub pad_tail: Option<String>,
}

/// Dolby Atmos IAB bitstream data-essence UL, as used in real Atmos DCP AuxData.
const ATMOS_DATA_TYPE_UL: &str = "urn:smpte:ul:060e2b34.04010105.0e090604.00000000";

/// Sorted J2K codestream paths in a directory (frame order).
fn sorted_j2k_frames(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut frames: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .map(|x| x.to_ascii_lowercase())
                .is_some_and(|x| x == "j2c" || x == "j2k")
        })
        .collect();
    frames.sort();
    frames
}

/// Create a complete DCP from the given configuration.
///
/// This orchestrates the full DCP creation pipeline:
/// 1. Wrap J2K frames and audio into MXF
/// 2. Generate CPL, PKL, ASSETMAP
/// 3. Optionally encrypt
pub fn create_dcp(config: &DcpConfig) -> i32 {
    tracing::info!(
        "Creating DCP: {} ({})",
        config.title,
        if config.standard == crate::Standard::Smpte {
            "SMPTE"
        } else {
            "Interop"
        }
    );

    let Some(j2k_dir) = config.j2k_dir.as_ref() else {
        tracing::error!("A J2K input directory is required");
        return -1;
    };
    if !j2k_dir.is_dir() {
        tracing::error!("J2K input directory does not exist: {}", j2k_dir.display());
        return -1;
    }

    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        tracing::error!("Failed to create output directory: {e}");
        return -1;
    }

    // Fail early if the essence won't fit: the wrapped MXFs are ~the size of the
    // J2K frames plus audio/atmos, so check that against the output filesystem.
    let mut required = crate::free_space::path_size(j2k_dir);
    if let Some(dir) = config.right_eye_dir.as_ref() {
        required += crate::free_space::path_size(dir);
    }
    for extra in [config.audio_path.as_ref(), config.atmos_path.as_ref()]
        .into_iter()
        .flatten()
    {
        required += crate::free_space::path_size(extra);
    }
    if let Err(e) = crate::free_space::check_destination_space(&config.output_dir, required) {
        tracing::error!("{e}");
        return -1;
    }

    // Fail before doing any work if we'd have nowhere safe to put the keys.
    if config.encrypt && config.key_out.is_none() {
        tracing::error!(
            "--key-out is required when encrypting; keys are never written next to the DCP"
        );
        return -1;
    }

    let fps = if config.frame_rate_num > 0 {
        config.frame_rate_num
    } else {
        24
    };

    // reject an illegal frame rate / resolution combo before wrapping anything
    let is_4k = config.resolution == crate::Resolution::FourK;
    if let Err(e) =
        crate::hfr::validate_fps_resolution(fps, is_4k, config.standard == crate::Standard::Smpte)
    {
        tracing::error!("{e}");
        return -1;
    }

    let stereoscopic = config.right_eye_dir.is_some();

    // ── Head/tail padding: parse durations and reject unsound combinations ──
    let head_frames = match config.pad_head.as_deref() {
        Some(spec) => match crate::pad::parse_pad_frames(spec, fps) {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("--pad-head: {e}");
                return -1;
            }
        },
        None => 0,
    };
    let tail_frames = match config.pad_tail.as_deref() {
        Some(spec) => match crate::pad::parse_pad_frames(spec, fps) {
            Ok(n) => n,
            Err(e) => {
                tracing::error!("--pad-tail: {e}");
                return -1;
            }
        },
        None => 0,
    };
    let padding = head_frames + tail_frames > 0;
    if padding {
        if config.reel_length_minutes > 0 {
            tracing::error!(
                "head/tail padding is not supported with reel splitting (--reel-length)"
            );
            return -1;
        }
        if stereoscopic {
            tracing::error!("head/tail padding is not supported with stereoscopic 3D");
            return -1;
        }
        if config.atmos_path.is_some() {
            tracing::error!(
                "head/tail padding is not supported with Atmos: the auxiliary track cannot be re-timed soundly this pass"
            );
            return -1;
        }
        // supplied SMPTE XML carries authored timing we will not rewrite; only SRT
        // (which we regenerate) can be shifted for head padding.
        let supplied_xml = config
            .subtitle_path
            .as_ref()
            .filter(|p| p.exists())
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("xml"));
        if head_frames > 0 && supplied_xml {
            tracing::error!(
                "head padding cannot re-time supplied SMPTE subtitle XML; supply SRT to shift, or pad only the tail"
            );
            return -1;
        }
    }

    let prepared_audio = match config.audio_path.as_ref().filter(|path| path.exists()) {
        Some(path) => {
            let output = config
                .output_dir
                .join(format!(".dcpwizard_audio_{}.wav", uuid::Uuid::new_v4()));
            match crate::mxf_wrap::prepare_51_audio(path, &output, config.audio_input_order) {
                Ok(true) => Some(output),
                Ok(false) => None,
                Err(e) => {
                    tracing::error!("audio preparation failed: {e}");
                    return -1;
                }
            }
        }
        None => None,
    };

    // multi-reel path is opt-in; the single-reel path below is unchanged
    if config.reel_length_minutes > 0 {
        if stereoscopic || config.atmos_path.is_some() {
            tracing::error!("stereoscopic 3D and Atmos are not supported with reel splitting");
            return -1;
        }
        let mut reel_config = config.clone();
        if let Some(path) = prepared_audio.as_ref() {
            reel_config.audio_path = Some(path.clone());
        }
        let code = crate::reel::create_multi_reel_dcp(&reel_config, fps);
        if let Some(path) = prepared_audio {
            let _ = std::fs::remove_file(path);
        }
        return code;
    }

    // ── Wrap picture MXF ──────────────────────────────────────────────
    let picture_uuid = uuid::Uuid::new_v4().to_string();
    let picture_mxf_name = format!("picture_{picture_uuid}.mxf");
    let picture_mxf_path = config.output_dir.join(&picture_mxf_name);
    // set from the (left-eye) frame count in the wrap block below
    let picture_duration: u64;

    // Mint content keys up front so the essence is encrypted at wrap time and
    // the PKL/ASSETMAP hashes below are taken from the final encrypted files.
    let picture_key = if config.encrypt {
        match crate::encrypt::generate_content_key(crate::encrypt::KeyType::Mdik, &picture_uuid) {
            Ok(k) => Some(k),
            Err(e) => {
                tracing::error!("content key generation failed: {e}");
                return -1;
            }
        }
    } else {
        None
    };

    {
        let left_frames = sorted_j2k_frames(j2k_dir);
        let content_count = left_frames.len() as u64;
        if content_count == 0 {
            tracing::error!("J2K input directory contains no codestreams");
            return -1;
        }

        let encryption = picture_key
            .as_ref()
            .map(|k| postkit::mxf_wrap::MxfEncryption {
                content_key: k.key,
                key_id: k.key_id,
            });

        if padding {
            // encode one black frame at the content's pixel dimensions, then repeat
            // its codestream for every padded frame (frame-wrapped MXF reuses it)
            let (bw, bh) = match crate::pad::read_j2k_dimensions(&left_frames[0]) {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("{e}");
                    return -1;
                }
            };
            let black = config
                .output_dir
                .join(format!(".dcpwizard_black_{picture_uuid}.j2c"));
            if let Err(e) = crate::pad::generate_black_frame(bw, bh, fps, &black) {
                tracing::error!("{e}");
                return -1;
            }
            let mut files =
                Vec::with_capacity(head_frames as usize + left_frames.len() + tail_frames as usize);
            files.extend(std::iter::repeat_n(black.clone(), head_frames as usize));
            files.extend(left_frames.iter().cloned());
            files.extend(std::iter::repeat_n(black.clone(), tail_frames as usize));
            picture_duration = files.len() as u64;
            let wrapped = crate::mxf_wrap::wrap_mxf_files(
                files,
                &picture_mxf_path,
                crate::mxf_wrap::MxfType::J2kPicture,
                fps,
                encryption,
                None,
            );
            let _ = std::fs::remove_file(&black);
            if wrapped.is_none() {
                tracing::error!("Failed to wrap padded picture MXF");
                return -1;
            }
            tracing::info!(
                "Picture MXF: {picture_mxf_name} ({picture_duration} frames: {head_frames} head + {content_count} content + {tail_frames} tail)"
            );
        } else if stereoscopic {
            picture_duration = content_count;
            // left eye is j2k_dir, right eye its own dir; both must match frame counts
            let right_dir = config.right_eye_dir.as_ref().unwrap();
            let right_frames = sorted_j2k_frames(right_dir);
            if right_frames.len() as u64 != picture_duration {
                tracing::error!(
                    "3D eye frame count mismatch: left={picture_duration}, right={}",
                    right_frames.len()
                );
                return -1;
            }
            // ST 429-10 legal 3D rates: the essence runs at 2x the edit rate, so
            // the composition edit rate must itself be a legal DCP rate.
            if crate::mxf_wrap::wrap_stereoscopic_files(
                left_frames,
                right_frames,
                &picture_mxf_path,
                fps,
                encryption,
            )
            .is_none()
            {
                tracing::error!("Failed to wrap stereoscopic picture MXF");
                return -1;
            }
            tracing::info!(
                "Stereoscopic picture MXF: {picture_mxf_name} ({picture_duration} frame pairs)"
            );
        } else {
            picture_duration = content_count;
            let wrap_config = crate::mxf_wrap::MxfWrapConfig {
                input_path: j2k_dir.clone(),
                output_mxf: picture_mxf_path.clone(),
                mxf_type: crate::mxf_wrap::MxfType::J2kPicture,
                frame_rate: fps,
                encryption,
                mca_config: None,
            };
            if crate::mxf_wrap::wrap_mxf(&wrap_config) != 0 {
                tracing::error!("Failed to wrap picture MXF");
                return -1;
            }
            tracing::info!("Picture MXF: {picture_mxf_name} ({picture_duration} frames)");
        }
    }

    // ── Wrap sound MXF ────────────────────────────────────────────────
    let sound_uuid = uuid::Uuid::new_v4().to_string();
    let sound_mxf_name = format!("sound_{sound_uuid}.mxf");
    let sound_mxf_path = config.output_dir.join(&sound_mxf_name);
    let mut has_sound = false;
    let sound_duration = picture_duration; // match picture duration
    let mut sound_key = None;
    // sound layout for the SMPTE CompositionMetadataAsset (ST 429-16)
    let mut main_sound = None;

    if let Some(audio_path) = prepared_audio.as_ref().or(config.audio_path.as_ref())
        && audio_path.exists()
    {
        sound_key = if config.encrypt {
            match crate::encrypt::generate_content_key(crate::encrypt::KeyType::Mdak, &sound_uuid) {
                Ok(k) => Some(k),
                Err(e) => {
                    tracing::error!("content key generation failed: {e}");
                    return -1;
                }
            }
        } else {
            None
        };
        // derive ST 429-12 MCA labels from the probed channel count plus any
        // HI/VI accessibility channel flags
        let channels = match crate::mxf_wrap::wav_channels(audio_path) {
            Ok(ch) => ch as u32,
            Err(e) => {
                tracing::error!("{e}");
                return -1;
            }
        };
        let mca_config =
            crate::mxf_wrap::build_mca_config(channels, config.hi_channel, config.vi_channel);
        // MainSoundConfiguration for the CPL metadata asset, from the same channel
        // count as the MCA labels (silent fill channels become '-').
        if let Some(configuration) =
            crate::cpl::main_sound_configuration(channels, config.hi_channel, config.vi_channel)
        {
            let sample_rate = crate::mxf_wrap::wav_sample_rate(audio_path).unwrap_or(48000);
            main_sound = Some(crate::cpl::MainSound {
                configuration,
                sample_rate,
            });
        }
        // when padding, extend the PCM with silence so the sound stays aligned
        // with the padded picture (sample-accurate at frame edges)
        let mut padded_audio: Option<PathBuf> = None;
        let wrap_source = if padding {
            let sample_rate = crate::mxf_wrap::wav_sample_rate(audio_path).unwrap_or(48000);
            if !sample_rate.is_multiple_of(fps) {
                tracing::error!(
                    "audio {sample_rate} Hz is not an integer number of samples per {fps} fps frame; cannot pad sample-accurately"
                );
                return -1;
            }
            let spf = (sample_rate / fps) as u64;
            let out = config
                .output_dir
                .join(format!(".dcpwizard_padded_{sound_uuid}.wav"));
            if let Err(e) = crate::pad::pad_wav_with_silence(
                audio_path,
                head_frames * spf,
                tail_frames * spf,
                &out,
            ) {
                tracing::error!("audio padding failed: {e}");
                return -1;
            }
            padded_audio = Some(out.clone());
            out
        } else {
            audio_path.clone()
        };
        let wrap_config = crate::mxf_wrap::MxfWrapConfig {
            input_path: wrap_source,
            output_mxf: sound_mxf_path.clone(),
            mxf_type: crate::mxf_wrap::MxfType::PcmAudio,
            frame_rate: fps,
            encryption: sound_key
                .as_ref()
                .map(|k| postkit::mxf_wrap::MxfEncryption {
                    content_key: k.key,
                    key_id: k.key_id,
                }),
            mca_config,
        };
        let wrap_code = crate::mxf_wrap::wrap_mxf(&wrap_config);
        if let Some(tmp) = padded_audio {
            let _ = std::fs::remove_file(tmp);
        }
        if wrap_code != 0 {
            tracing::error!("Failed to wrap sound MXF");
            return -1;
        }
        has_sound = true;
        tracing::info!("Sound MXF: {sound_mxf_name}");
    }

    // ── Wrap subtitle (SMPTE timed text) MXF ──────────────────────────
    let subtitle_uuid = uuid::Uuid::new_v4().to_string();
    let subtitle_mxf_name = format!("subtitle_{subtitle_uuid}.mxf");
    let subtitle_mxf_path = config.output_dir.join(&subtitle_mxf_name);
    let mut has_subtitle = false;
    let mut subtitle_duration = 0u64;
    let subtitle_lang = if config.subtitle_language.is_empty() {
        "en"
    } else {
        &config.subtitle_language
    };

    if let Some(subtitle_path) = config.subtitle_path.as_ref()
        && subtitle_path.exists()
    {
        let is_xml = subtitle_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
        // Preserve authored SMPTE XML, including its placement and styling. SRT
        // remains a centered-bottom conversion because it carries no placement.
        let dcst_path = if is_xml {
            subtitle_path.clone()
        } else {
            let path = config
                .output_dir
                .join(format!("subtitle_{subtitle_uuid}.xml"));
            // head padding shifts the program, so slide SRT cues by head_frames
            if let Err(e) = crate::subtitle::srt_to_shifted_dcst(
                subtitle_path,
                head_frames,
                subtitle_lang,
                fps,
                &path,
            ) {
                tracing::error!("Subtitle conversion failed: {e}");
                return -1;
            }
            path
        };
        let wrap_config = crate::mxf_wrap::MxfWrapConfig {
            input_path: dcst_path.clone(),
            output_mxf: subtitle_mxf_path.clone(),
            mxf_type: crate::mxf_wrap::MxfType::TimedText,
            frame_rate: fps,
            encryption: None,
            mca_config: None,
        };
        let Some(track) = crate::mxf_wrap::wrap_mxf_result(&wrap_config) else {
            tracing::error!("Failed to wrap subtitle MXF");
            return -1;
        };
        subtitle_duration = track.duration;
        // The generated DCST now lives inside the MXF. Never remove a supplied
        // subtitle XML file.
        if !is_xml {
            let _ = std::fs::remove_file(&dcst_path);
        }
        has_subtitle = true;
        tracing::info!("Subtitle MXF: {subtitle_mxf_name}");
    }

    // ── Wrap Atmos / DCData auxiliary MXF (ST 429-18) ─────────────────
    let atmos_uuid = uuid::Uuid::new_v4().to_string();
    let atmos_mxf_name = format!("atmos_{atmos_uuid}.mxf");
    let atmos_mxf_path = config.output_dir.join(&atmos_mxf_name);
    let mut aux_data: Option<crate::cpl::AuxData> = None;

    if let Some(ref atmos_path) = config.atmos_path {
        if !atmos_path.exists() {
            tracing::error!("Atmos input not found: {}", atmos_path.display());
            return -1;
        }
        // aux data carries the essence unencrypted; the synthetic id keeps
        // CPL/PKL/ASSETMAP consistent (same pattern as picture/sound).
        let wrap_config = crate::mxf_wrap::MxfWrapConfig {
            input_path: atmos_path.clone(),
            output_mxf: atmos_mxf_path.clone(),
            mxf_type: crate::mxf_wrap::MxfType::Atmos,
            frame_rate: fps,
            encryption: None,
            mca_config: None,
        };
        let Some(track) = crate::mxf_wrap::wrap_mxf_result(&wrap_config) else {
            tracing::error!("Failed to wrap Atmos MXF");
            return -1;
        };
        // one input file = one frame; a track shorter than the picture would
        // produce a broken DCP, so refuse instead of writing it
        if track.duration != picture_duration {
            tracing::error!(
                "Atmos track is {} frames but the picture is {picture_duration}; \
                 pass a directory with one Atmos frame file per picture frame",
                track.duration
            );
            return -1;
        }
        aux_data = Some(crate::cpl::AuxData {
            id: atmos_uuid.clone(),
            edit_rate_num: fps,
            edit_rate_den: 1,
            duration: track.duration,
            entry_point: 0,
            key_id: None,
            data_type: ATMOS_DATA_TYPE_UL.to_string(),
        });
        tracing::info!("Atmos MXF: {atmos_mxf_name} ({} frames)", track.duration);
    }

    // ── Generate CPL ──────────────────────────────────────────────────
    let cpl_uuid = uuid::Uuid::new_v4().to_string();
    let pkl_uuid = uuid::Uuid::new_v4().to_string();

    // honour an explicit scope/flat container; else the full-container preset
    let (pic_w, pic_h) = if config.container_width > 0 && config.container_height > 0 {
        (config.container_width, config.container_height)
    } else {
        (config.resolution.width(), config.resolution.height())
    };

    let reel = crate::cpl::CplReel {
        reel_id: uuid::Uuid::new_v4().to_string(),
        picture_id: picture_uuid.clone(),
        picture_width: pic_w,
        picture_height: pic_h,
        picture_edit_rate_num: fps,
        picture_edit_rate_den: 1,
        picture_duration,
        picture_entry_point: 0,
        picture_key_id: picture_key.as_ref().map(|k| k.info.key_id.clone()),
        sound_id: if has_sound {
            Some(sound_uuid.clone())
        } else {
            None
        },
        sound_edit_rate_num: fps,
        sound_edit_rate_den: 1,
        sound_duration,
        sound_entry_point: 0,
        sound_key_id: sound_key.as_ref().map(|k| k.info.key_id.clone()),
        subtitle_id: if has_subtitle {
            Some(subtitle_uuid.clone())
        } else {
            None
        },
        subtitle_edit_rate_num: fps,
        subtitle_edit_rate_den: 1,
        subtitle_duration,
        subtitle_entry_point: 0,
        subtitle_language: if has_subtitle {
            Some(subtitle_lang.to_string())
        } else {
            None
        },
        stereoscopic,
        aux_data: aux_data.clone(),
    };

    let cpl_path = config.output_dir.join(format!("CPL_{cpl_uuid}.xml"));
    let cpl_config = crate::cpl::CplConfig {
        title: config.title.clone(),
        content_kind: config.content_type.as_cpl_kind().into(),
        reels: vec![reel],
        standard: config.standard,
        main_sound,
        ..Default::default()
    };
    if crate::cpl::generate_cpl(&cpl_config, &cpl_uuid, &cpl_path) != 0 {
        tracing::error!("Failed to generate CPL");
        return -1;
    }

    // ── Generate PKL ──────────────────────────────────────────────────
    let pkl_path = config.output_dir.join(format!("PKL_{pkl_uuid}.xml"));
    let cpl_hash = crate::hash::hash_file(&cpl_path).unwrap_or_default();
    let cpl_size = std::fs::metadata(&cpl_path).map(|m| m.len()).unwrap_or(0);
    let mut pkl_entries = vec![crate::pkl::PklEntry {
        id: cpl_uuid.clone(),
        asset_type: "text/xml".into(),
        file: cpl_path.clone(),
        hash: cpl_hash,
        size: cpl_size,
    }];

    let pic_hash = crate::hash::hash_file(&picture_mxf_path).unwrap_or_default();
    let pic_size = std::fs::metadata(&picture_mxf_path)
        .map(|m| m.len())
        .unwrap_or(0);
    pkl_entries.push(crate::pkl::PklEntry {
        id: picture_uuid.clone(),
        asset_type: "application/mxf".into(),
        file: picture_mxf_path.clone(),
        hash: pic_hash,
        size: pic_size,
    });
    if has_sound {
        let snd_hash = crate::hash::hash_file(&sound_mxf_path).unwrap_or_default();
        let snd_size = std::fs::metadata(&sound_mxf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        pkl_entries.push(crate::pkl::PklEntry {
            id: sound_uuid.clone(),
            asset_type: "application/mxf".into(),
            file: sound_mxf_path.clone(),
            hash: snd_hash,
            size: snd_size,
        });
    }
    if has_subtitle {
        let sub_hash = crate::hash::hash_file(&subtitle_mxf_path).unwrap_or_default();
        let sub_size = std::fs::metadata(&subtitle_mxf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        pkl_entries.push(crate::pkl::PklEntry {
            id: subtitle_uuid.clone(),
            asset_type: "application/mxf".into(),
            file: subtitle_mxf_path.clone(),
            hash: sub_hash,
            size: sub_size,
        });
    }
    if aux_data.is_some() {
        let aux_hash = crate::hash::hash_file(&atmos_mxf_path).unwrap_or_default();
        let aux_size = std::fs::metadata(&atmos_mxf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        pkl_entries.push(crate::pkl::PklEntry {
            id: atmos_uuid.clone(),
            asset_type: "application/mxf".into(),
            file: atmos_mxf_path.clone(),
            hash: aux_hash,
            size: aux_size,
        });
    }

    if crate::pkl::generate_pkl(&pkl_entries, &pkl_uuid, config.standard, &pkl_path) != 0 {
        tracing::error!("Failed to generate PKL");
        return -1;
    }

    // ── Generate ASSETMAP ─────────────────────────────────────────────
    let mut am_entries = vec![
        crate::assetmap::AssetMapEntry {
            id: pkl_uuid,
            path: pkl_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            packing_list: true,
        },
        crate::assetmap::AssetMapEntry {
            id: cpl_uuid.clone(),
            path: cpl_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            packing_list: false,
        },
    ];
    am_entries.push(crate::assetmap::AssetMapEntry {
        id: picture_uuid,
        path: picture_mxf_name,
        packing_list: false,
    });
    if has_sound {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: sound_uuid,
            path: sound_mxf_name,
            packing_list: false,
        });
    }
    if has_subtitle {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: subtitle_uuid,
            path: subtitle_mxf_name,
            packing_list: false,
        });
    }
    if aux_data.is_some() {
        am_entries.push(crate::assetmap::AssetMapEntry {
            id: atmos_uuid,
            path: atmos_mxf_name,
            packing_list: false,
        });
    }

    if crate::assetmap::generate_assetmap(&am_entries, &config.output_dir, config.standard) != 0 {
        tracing::error!("Failed to generate ASSETMAP");
        return -1;
    }

    // Persist the content keys for the KDM step. The essence was already
    // encrypted at wrap time, so this only writes the keys file. The path is
    // caller-supplied (key_out); keys are never written next to the DCP.
    if config.encrypt {
        // Guaranteed Some by the early guard, but stay defensive: never fall
        // back to a default location for secret key material.
        let Some(keys_path) = config.key_out.as_ref() else {
            tracing::error!("--key-out is required when encrypting");
            return -1;
        };
        if let Some(parent) = keys_path.parent().filter(|p| !p.as_os_str().is_empty())
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::error!("Failed to create key-out directory: {e}");
            return -1;
        }
        let mut bundle = crate::encrypt::KeyBundle {
            cpl_id: cpl_uuid.clone(),
            keys: Vec::new(),
        };
        if let Some(k) = picture_key {
            bundle.keys.push(k.info);
        }
        if let Some(k) = sound_key {
            bundle.keys.push(k.info);
        }
        if let Err(e) = bundle.write(keys_path) {
            tracing::error!("Failed to write keys file: {e}");
            return -1;
        }
        tracing::warn!(
            "Wrote content keys to {} — this file holds the plaintext AES keys. \
             Keep it secret, do not ship it in the DCP, and use it only to build KDMs.",
            keys_path.display()
        );
    }

    tracing::info!("DCP created: {}", config.output_dir.display());
    if let Some(path) = prepared_audio {
        let _ = std::fs::remove_file(path);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dcp_requires_picture_input() {
        let dir = tempfile::tempdir().unwrap();
        let config = DcpConfig {
            title: "Test Film".into(),
            output_dir: dir.path().to_path_buf(),
            frame_rate_num: 24,
            frame_rate_den: 1,
            ..Default::default()
        };
        assert_eq!(create_dcp(&config), -1);
        assert!(!dir.path().join("ASSETMAP.xml").exists());
    }

    #[test]
    fn create_dcp_encrypt_requires_key_out() {
        let dir = tempfile::tempdir().unwrap();
        let j2k_dir = dir.path().join("j2k");
        std::fs::create_dir_all(&j2k_dir).unwrap();
        let config = DcpConfig {
            title: "Test Film".into(),
            output_dir: dir.path().join("out"),
            j2k_dir: Some(j2k_dir),
            encrypt: true,
            key_out: None,
            frame_rate_num: 24,
            frame_rate_den: 1,
            ..Default::default()
        };
        // must fail before wrapping, and must not write keys anywhere
        assert_eq!(create_dcp(&config), -1);
        assert!(!dir.path().join("out/KEYS.json").exists());
    }
}
