//! The ISDCF content title for a `DcpConfig`: the one place the packaging
//! configuration is mapped onto [`crate::isdcf_name`]'s input, shared by the CLI
//! and the GUI so both name a package identically.

use crate::dcp::DcpConfig;
use crate::isdcf_name::{
    DEFAULT_CONTAINER_SIZE, IsdcfDate, IsdcfNameInput, SoundtrackChannel, TerritoryType, TextKind,
    isdcf_name,
};
use chrono::Datelike;
use std::path::{Path, PathBuf};

/// Composition version number when the config carries none, as Bv2.1 requires
/// the element present.
const DEFAULT_VERSION_NUMBER: u32 = 1;

/// The parts of the name that are not packaging configuration: who mastered it,
/// what kind of version it is, and when.
#[derive(Debug, Clone, Default)]
pub struct IsdcfNamingOptions {
    pub studio: Option<String>,
    pub temp_version: bool,
    pub pre_release: bool,
    pub red_band: bool,
    pub two_d_version_of_three_d: bool,
    pub territory_type: TerritoryType,
    /// Creation date in the name. None is today, UTC.
    pub date: Option<IsdcfDate>,
    pub version_file: bool,
}

/// The packaged soundtrack as the name counts it.
#[derive(Debug, Clone, Default)]
pub struct SoundtrackSummary {
    pub channels: Vec<SoundtrackChannel>,
    pub has_hearing_impaired: bool,
    pub has_visually_impaired: bool,
}

/// The soundtrack the packaged channel count carries, read the way the CPL's
/// `main_sound_configuration` reads it: HI and VI sit outside the main layout,
/// and a count with no canonical DCP layout carries no channels at all.
pub fn soundtrack_summary(
    channel_count: usize,
    hi_channel: Option<u32>,
    vi_channel: Option<u32>,
) -> SoundtrackSummary {
    let accessibility = hi_channel.is_some() as usize + vi_channel.is_some() as usize;
    let main_count = channel_count.saturating_sub(accessibility);
    let channels = match main_count {
        2 => vec![SoundtrackChannel::Left, SoundtrackChannel::Right],
        6 | 16 => vec![
            SoundtrackChannel::Left,
            SoundtrackChannel::Right,
            SoundtrackChannel::Centre,
            SoundtrackChannel::Lfe,
            SoundtrackChannel::LeftSurround,
            SoundtrackChannel::RightSurround,
        ],
        8 => vec![
            SoundtrackChannel::Left,
            SoundtrackChannel::Right,
            SoundtrackChannel::Centre,
            SoundtrackChannel::Lfe,
            SoundtrackChannel::LeftSurround,
            SoundtrackChannel::RightSurround,
            SoundtrackChannel::BackSurroundLeft,
            SoundtrackChannel::BackSurroundRight,
        ],
        _ => Vec::new(),
    };
    SoundtrackSummary {
        channels,
        has_hearing_impaired: hi_channel.is_some(),
        has_visually_impaired: vi_channel.is_some(),
    }
}

// where a build's sound comes from, read before any of it is prepared
#[derive(Debug, Clone, Copy)]
pub struct SoundtrackSource<'a> {
    // a WAV, or a directory of channel WAVs
    pub audio: Option<&'a Path>,
    // mono WAVs routed by their channel suffix, in place of audio
    pub channel_files: Option<&'a [PathBuf]>,
    pub picture: Option<&'a Path>,
    pub audio_map: Option<&'a str>,
    pub upmix: bool,
    pub hi_channel: Option<u32>,
    pub vi_channel: Option<u32>,
}

// the named sound, else the picture's own, through the map and the upmix the
// build applies to it
pub fn build_soundtrack(source: &SoundtrackSource) -> Result<SoundtrackSummary, String> {
    let channel_count = prepared_channel_count(source)?;
    Ok(soundtrack_summary(
        channel_count,
        source.hi_channel,
        source.vi_channel,
    ))
}

fn prepared_channel_count(source: &SoundtrackSource) -> Result<usize, String> {
    let channel_set = source
        .channel_files
        .map(crate::audio_route::channel_lanes)
        .transpose()?;
    let read_count = match (&channel_set, source.audio) {
        (Some(routed), _) => crate::audio_route::routed_channel_count_of(routed),
        (None, Some(dir)) if dir.is_dir() => crate::audio_route::routed_channel_count(dir)?,
        (None, Some(wav)) => usize::from(crate::mxf_wrap::wav_channels(wav)?),
        (None, None) => embedded_channel_count(source.picture)?,
    };
    if read_count == 0 {
        return Ok(0);
    }
    // the build routes a channel directory after the map has run
    let routes_a_directory = channel_set.is_none() && source.audio.is_some_and(Path::is_dir);
    let channel_set_map = match (&channel_set, source.audio_map) {
        (Some(routed), Some(spec)) => Some(crate::audio_map::channel_set_audio_map(spec, routed)?),
        _ => None,
    };
    let mapped_count = match channel_set_map.as_deref().or(source.audio_map) {
        Some(spec) if !routes_a_directory => {
            crate::audio_map::parse_audio_map(spec, read_count)?.output_channels()
        }
        _ => read_count,
    };
    if source.upmix {
        return Ok(crate::mxf_wrap::CANONICAL_51_CHANNELS as usize);
    }
    Ok(mapped_count)
}

const FFPROBE_DEFAULT_DISPOSITION: &str = "1";

// the stream ffmpeg extracts with no map: a default one first, then the widest
fn embedded_channel_count(picture: Option<&Path>) -> Result<usize, String> {
    let Some(picture) = picture.filter(|picture| picture.is_file()) else {
        return Ok(0);
    };
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=channels:stream_disposition=default",
            "-of",
            "csv=p=0",
        ])
        .arg(picture)
        .output()
        .map_err(|e| format!("failed to run ffprobe on {}: {e}", picture.display()))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe could not read the sound in {}: {}",
            picture.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let streams = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (channels, default) = line.trim().split_once(',')?;
            Some((
                default == FFPROBE_DEFAULT_DISPOSITION,
                channels.parse::<usize>().ok()?,
            ))
        })
        .collect::<Vec<_>>();
    // max keeps the last of equals, so the first stream wins a tie
    Ok(streams
        .into_iter()
        .rev()
        .max()
        .map_or(0, |(_, channels)| channels))
}

// what the ISDCF name of a package is built from
#[derive(Debug, Clone, Copy)]
pub struct IsdcfNaming<'a> {
    pub config: &'a DcpConfig,
    pub options: &'a IsdcfNamingOptions,
    pub sound: SoundtrackSource<'a>,
    pub burnt_in_subtitle: bool,
}

impl IsdcfNaming<'_> {
    pub fn name(&self) -> Result<String, String> {
        let date = self.options.date.unwrap_or_else(today);
        self.name_with_date(Some(date))
    }

    pub fn name_with_date(&self, date: Option<IsdcfDate>) -> Result<String, String> {
        let sound = build_soundtrack(&self.sound)?;
        Ok(isdcf_name(&name_input(
            self.config,
            self.options,
            &sound,
            self.burnt_in_subtitle,
            date,
        )))
    }
}

const ISDCF_DATE_DIGITS: usize = 8;

// the date in a folder that is undated_folder_name with a date put back in
pub fn date_apart_from(folder_name: &str, undated_folder_name: &str) -> Option<IsdcfDate> {
    folder_name.match_indices('_').find_map(|(start, _)| {
        let digits = folder_name.get(start + 1..start + 1 + ISDCF_DATE_DIGITS)?;
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let rest = &folder_name[start + 1 + ISDCF_DATE_DIGITS..];
        if format!("{}{rest}", &folder_name[..start]) != undated_folder_name {
            return None;
        }
        Some(IsdcfDate {
            year: digits[..4].parse().ok()?,
            month: digits[4..6].parse().ok()?,
            day: digits[6..].parse().ok()?,
        })
    })
}

/// The ISDCF content title for a package. `burnt_in_subtitle` says the subtitles
/// are drawn into the picture, which the name spells in lower case.
pub fn isdcf_title(
    config: &DcpConfig,
    options: &IsdcfNamingOptions,
    sound: &SoundtrackSummary,
    burnt_in_subtitle: bool,
) -> String {
    let date = options.date.unwrap_or_else(today);
    isdcf_name(&name_input(
        config,
        options,
        sound,
        burnt_in_subtitle,
        Some(date),
    ))
}

fn name_input(
    config: &DcpConfig,
    options: &IsdcfNamingOptions,
    sound: &SoundtrackSummary,
    burnt_in_subtitle: bool,
    date: Option<IsdcfDate>,
) -> IsdcfNameInput {
    let (open_text_languages, open_text_burnt_in) = open_text(config, burnt_in_subtitle);
    let closed_text_languages = match config.ccap_path {
        Some(_) => vec![config.ccap_language.clone()],
        None => Vec::new(),
    };

    IsdcfNameInput {
        title: config.title.clone(),
        content_type: config.content_type,
        version_number: config.version_number.unwrap_or(DEFAULT_VERSION_NUMBER),
        content_versions: config.content_versions.clone(),
        temp_version: options.temp_version,
        pre_release: options.pre_release,
        red_band: options.red_band,
        chain: config.chain.clone(),
        three_d: config.stereo_3d,
        two_d_version_of_three_d: options.two_d_version_of_three_d,
        luminance: config.luminance.clone(),
        frame_rate: rounded_frame_rate(config.frame_rate_num, config.frame_rate_den),
        hdr: config.hdr_dci,
        container_size: container_size(config),
        // the CPL declares the stored area as the active one, so the name has no
        // interior aspect to spell
        active_picture_size: None,
        audio_language: config.audio_language.clone(),
        open_text_languages,
        open_text_kind: TextKind::Subtitle,
        open_text_burnt_in,
        closed_text_languages,
        closed_text_kind: TextKind::Caption,
        territory_type: options.territory_type,
        release_territory: config.release_territory.clone(),
        ratings: config.ratings.clone(),
        soundtrack_channels: sound.channels.clone(),
        has_hearing_impaired: sound.has_hearing_impaired,
        has_visually_impaired: sound.has_visually_impaired,
        has_atmos: config.atmos_path.is_some(),
        resolution: config.resolution,
        studio: options.studio.clone(),
        date,
        facility: config.facility.clone(),
        standard: config.standard,
        version_file: options.version_file,
    }
}

/// The container whose aspect the name spells. Without one the CPL declares the
/// coded raster as the active area, so the raster is the container.
fn container_size(config: &DcpConfig) -> (u32, u32) {
    if config.container_width > 0 && config.container_height > 0 {
        return (config.container_width, config.container_height);
    }
    config
        .j2k_dir
        .as_deref()
        .and_then(|dir| crate::cpl::picture_geometry(dir, 0, 0).ok())
        .map(|geometry| (geometry.stored_width, geometry.stored_height))
        .unwrap_or(DEFAULT_CONTAINER_SIZE)
}

/// The open text languages and whether they are burnt in. A registered subtitle
/// track wins over a burn, since the name spells the track the package carries.
fn open_text(config: &DcpConfig, burnt_in_subtitle: bool) -> (Vec<String>, bool) {
    if config.subtitle_path.is_some() {
        return (vec![config.subtitle_language.clone()], false);
    }
    if burnt_in_subtitle {
        return (vec![config.subtitle_language.clone()], true);
    }
    (Vec::new(), false)
}

fn rounded_frame_rate(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return numerator;
    }
    (numerator as f64 / denominator as f64).round() as u32
}

fn today() -> IsdcfDate {
    let now = chrono::Utc::now().date_naive();
    IsdcfDate {
        year: now.year() as u32,
        month: now.month(),
        day: now.day(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isdcf_name::Rating;
    use crate::{ContentType, Resolution, Standard};
    use std::path::PathBuf;

    const DATE: IsdcfDate = IsdcfDate {
        year: 2026,
        month: 8,
        day: 16,
    };

    fn config() -> DcpConfig {
        DcpConfig {
            title: "My Film".into(),
            standard: Standard::Smpte,
            resolution: Resolution::TwoK,
            content_type: ContentType::Test,
            frame_rate_num: 24,
            frame_rate_den: 1,
            container_width: 1998,
            container_height: 1080,
            audio_language: Some("en".into()),
            facility: Some("PPF".into()),
            ..Default::default()
        }
    }

    fn options() -> IsdcfNamingOptions {
        IsdcfNamingOptions {
            date: Some(DATE),
            ..Default::default()
        }
    }

    fn stereo() -> SoundtrackSummary {
        soundtrack_summary(2, None, None)
    }

    #[test]
    fn a_stereo_package_is_named_from_its_configuration() {
        assert_eq!(
            isdcf_title(&config(), &options(), &stereo(), false),
            "MyFilm_TST-1_F_EN-XX_20_2K_20260816_PPF_SMPTE_OV"
        );
    }

    #[test]
    fn the_channel_ladder_follows_the_packaged_count() {
        let ladder = [(2, "_20"), (6, "_51"), (16, "_51"), (8, "_71"), (3, "_MOS")];
        for (channel_count, expected) in ladder {
            let sound = soundtrack_summary(channel_count, None, None);
            let name = isdcf_title(&config(), &options(), &sound, false);
            assert!(
                name.contains(expected),
                "{channel_count} channels must name {expected}, got {name}"
            );
        }
    }

    #[test]
    fn accessibility_channels_sit_outside_the_main_layout() {
        let sound = soundtrack_summary(8, Some(6), Some(7));
        assert_eq!(sound.channels.len(), 6, "eight channels less HI and VI");
        let name = isdcf_title(&config(), &options(), &sound, false);
        assert!(name.contains("_51-HI-VI"), "{name}");
    }

    #[test]
    fn an_open_subtitle_is_upper_case_and_a_burnt_in_one_lower() {
        let open = DcpConfig {
            subtitle_path: Some(PathBuf::from("subs.srt")),
            subtitle_language: "fr".into(),
            ..config()
        };
        assert!(
            isdcf_title(&open, &options(), &stereo(), false).contains("_EN-FR_"),
            "an open subtitle track is spelled in upper case"
        );

        let burnt = DcpConfig {
            subtitle_language: "fr".into(),
            ..config()
        };
        assert!(
            isdcf_title(&burnt, &options(), &stereo(), true).contains("_EN-fr_"),
            "burnt-in subtitles are spelled in lower case"
        );
    }

    #[test]
    fn a_closed_caption_track_is_marked_ccap() {
        let captioned = DcpConfig {
            ccap_path: Some(PathBuf::from("captions.srt")),
            ccap_language: "de".into(),
            ..config()
        };
        assert!(
            isdcf_title(&captioned, &options(), &stereo(), false).contains("_EN-DE-CCAP_"),
            "a closed-caption track is marked CCAP"
        );
    }

    #[test]
    fn a_stereoscopic_package_is_marked_3d_twice() {
        let three_d = DcpConfig {
            stereo_3d: true,
            ..config()
        };
        let name = isdcf_title(&three_d, &options(), &stereo(), false);
        assert!(name.contains("_TST-1-3D_"), "{name}");
        assert!(name.ends_with("_SMPTE-3D_OV"), "{name}");
    }

    #[test]
    fn no_date_names_today() {
        let options = IsdcfNamingOptions::default();
        let name = isdcf_title(&config(), &options, &stereo(), false);
        let today = today();
        assert!(
            name.contains(&format!(
                "_{:04}{:02}{:02}_",
                today.year, today.month, today.day
            )),
            "{name}"
        );
    }

    #[test]
    fn no_container_takes_the_aspect_from_the_coded_raster() {
        let dir = tempfile::tempdir().unwrap();
        let frames = dir.path().join("frames");
        std::fs::create_dir_all(&frames).unwrap();
        crate::pad::generate_black_frame(2048, 858, 24, &frames.join("frame_00000.j2c"))
            .expect("encode frame");

        let config = DcpConfig {
            container_width: 0,
            container_height: 0,
            j2k_dir: Some(frames),
            ..config()
        };
        let name = isdcf_title(&config, &options(), &stereo(), false);
        assert!(name.contains("_S_"), "2048x858 frames are scope: {name}");
    }

    #[test]
    fn no_container_and_no_frames_fall_back_to_flat() {
        let config = DcpConfig {
            container_width: 0,
            container_height: 0,
            ..config()
        };
        let name = isdcf_title(&config, &options(), &stereo(), false);
        assert!(name.contains("_F_"), "{name}");
    }

    #[test]
    fn a_version_file_is_named_vf() {
        let options = IsdcfNamingOptions {
            version_file: true,
            ..options()
        };
        assert!(isdcf_title(&config(), &options, &stereo(), false).ends_with("_VF"));
    }

    /// The mapping is pinned against the name built by hand from the same facts.
    #[test]
    fn the_mapping_matches_a_hand_built_name() {
        let config = DcpConfig {
            title: "My Nice Film".into(),
            content_type: ContentType::Feature,
            version_number: Some(2),
            content_versions: vec!["Final Cut".into()],
            release_territory: Some("GB".into()),
            ratings: vec![Rating {
                agency: "http://www.bbfc.co.uk/BBFCRatings".into(),
                label: "PG".into(),
            }],
            chain: Some("MyChain".into()),
            atmos_path: Some(PathBuf::from("atmos.mxf")),
            ccap_path: Some(PathBuf::from("captions.srt")),
            ccap_language: "fr".into(),
            frame_rate_num: 48,
            frame_rate_den: 1,
            container_width: 2048,
            container_height: 858,
            resolution: Resolution::FourK,
            ..config()
        };
        let options = IsdcfNamingOptions {
            studio: Some("Disney".into()),
            temp_version: true,
            ..options()
        };
        let sound = soundtrack_summary(6, None, None);

        let expected = isdcf_name(&IsdcfNameInput {
            title: "My Nice Film".into(),
            content_type: ContentType::Feature,
            version_number: 2,
            content_versions: vec!["Final Cut".into()],
            temp_version: true,
            chain: Some("MyChain".into()),
            frame_rate: 48,
            container_size: (2048, 858),
            audio_language: Some("en".into()),
            closed_text_languages: vec!["fr".into()],
            closed_text_kind: TextKind::Caption,
            release_territory: Some("GB".into()),
            ratings: vec![Rating {
                agency: "http://www.bbfc.co.uk/BBFCRatings".into(),
                label: "PG".into(),
            }],
            soundtrack_channels: sound.channels.clone(),
            has_atmos: true,
            resolution: Resolution::FourK,
            studio: Some("Disney".into()),
            date: Some(DATE),
            facility: Some("PPF".into()),
            standard: Standard::Smpte,
            ..Default::default()
        });

        assert_eq!(isdcf_title(&config, &options, &sound, false), expected);
        assert_eq!(
            expected,
            "MyNiceFilm_FTR-2-Temp-MyChain-48_S_EN-FR-CCAP_GB-PG_51-IAB_4K_DISN_20260816_PPF_SMPTE_OV"
        );
    }

    // a picture with its own sound, one stream per channel layout, in order
    fn source_with_sound(dir: &Path, layouts: &[&str]) -> PathBuf {
        let path = dir.join("source.mkv");
        let mut command = std::process::Command::new("ffmpeg");
        command.args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=320x240:rate=24",
        ]);
        for layout in layouts {
            command.args([
                "-f",
                "lavfi",
                "-i",
                &format!("anullsrc=channel_layout={layout}:sample_rate=48000"),
            ]);
        }
        for index in 0..=layouts.len() {
            command.args(["-map", &index.to_string()]);
        }
        let made = command
            .args(["-t", "1", "-c:a", "pcm_s24le"])
            .arg(&path)
            .output()
            .expect("ffmpeg has to run");
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );
        path
    }

    fn write_wav(path: &Path, channels: u16) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for _ in 0..channels {
            writer.write_sample(0).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn sound_from<'a>(audio: Option<&'a Path>, picture: &'a Path) -> SoundtrackSource<'a> {
        SoundtrackSource {
            audio,
            channel_files: None,
            picture: Some(picture),
            audio_map: None,
            upmix: false,
            hi_channel: None,
            vi_channel: None,
        }
    }

    fn name_of(sound: SoundtrackSource) -> String {
        let config = config();
        let options = options();
        IsdcfNaming {
            config: &config,
            options: &options,
            sound,
            burnt_in_subtitle: false,
        }
        .name()
        .unwrap()
    }

    #[test]
    fn with_no_wav_the_picture_sound_names_the_channels() {
        let dir = tempfile::tempdir().unwrap();
        let source = source_with_sound(dir.path(), &["5.1"]);
        let name = name_of(sound_from(None, &source));
        assert!(name.contains("_EN-XX_51_"), "{name}");
    }

    #[test]
    fn a_picked_wav_names_the_channels_over_the_picture_sound() {
        let dir = tempfile::tempdir().unwrap();
        let source = source_with_sound(dir.path(), &["5.1"]);
        let wav = dir.path().join("stereo.wav");
        write_wav(&wav, 2);
        let name = name_of(sound_from(Some(&wav), &source));
        assert!(name.contains("_EN-XX_20_"), "{name}");
    }

    #[test]
    fn a_picture_with_no_sound_and_no_wav_is_mos() {
        let dir = tempfile::tempdir().unwrap();
        let source = source_with_sound(dir.path(), &[]);
        assert!(name_of(sound_from(None, &source)).contains("_MOS_"));
        assert!(name_of(sound_from(None, dir.path())).contains("_MOS_"));
    }

    #[test]
    fn the_picture_sound_is_the_stream_ffmpeg_extracts() {
        let dir = tempfile::tempdir().unwrap();
        let source = source_with_sound(dir.path(), &["stereo", "5.1"]);
        let extracted = crate::audio_fallback::extract_embedded_audio(&source, dir.path())
            .unwrap()
            .expect("the source has sound");
        let extracted_channels = crate::mxf_wrap::wav_channels(&extracted).unwrap();
        assert_eq!(
            embedded_channel_count(Some(&source)).unwrap(),
            usize::from(extracted_channels)
        );
    }

    #[test]
    fn the_upmix_and_the_map_name_what_they_produce() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("stereo.wav");
        write_wav(&wav, 2);
        let upmixed = SoundtrackSource {
            upmix: true,
            ..sound_from(Some(&wav), dir.path())
        };
        assert!(name_of(upmixed).contains("_51_"));
        let mapped = SoundtrackSource {
            audio_map: Some("1:L,2:R,1:C,2:LFE,1:Ls,2:Rs"),
            ..sound_from(Some(&wav), dir.path())
        };
        assert!(name_of(mapped).contains("_51_"));
    }

    #[test]
    fn a_channel_directory_names_the_lanes_it_routes() {
        let dir = tempfile::tempdir().unwrap();
        for lane in ["L", "R", "C", "Lfe", "Ls", "Rs"] {
            write_wav(&dir.path().join(format!("mix_{lane}.wav")), 1);
        }
        let name = name_of(sound_from(Some(dir.path()), dir.path()));
        assert!(name.contains("_51_"), "{name}");
    }

    #[test]
    fn a_channel_set_names_the_lanes_it_routes() {
        let dir = tempfile::tempdir().unwrap();
        let files: Vec<PathBuf> = ["L", "R", "C", "LFE", "Ls", "Rs"]
            .iter()
            .map(|lane| dir.path().join(format!("mix.{lane}.wav")))
            .collect();
        for file in &files {
            write_wav(file, 1);
        }
        let name = name_of(SoundtrackSource {
            channel_files: Some(&files),
            ..sound_from(None, dir.path())
        });
        assert!(name.contains("_51_"), "{name}");
    }

    #[test]
    fn a_channel_set_map_names_what_it_produces_from_the_routed_lanes() {
        let dir = tempfile::tempdir().unwrap();
        let files: Vec<PathBuf> = ["L", "Rs"]
            .iter()
            .map(|lane| dir.path().join(format!("mix_{lane}.wav")))
            .collect();
        for file in &files {
            write_wav(file, 1);
        }
        let routed = name_of(SoundtrackSource {
            channel_files: Some(&files),
            ..sound_from(None, dir.path())
        });
        assert!(routed.contains("_51_"), "{routed}");
        let folded = name_of(SoundtrackSource {
            channel_files: Some(&files),
            audio_map: Some("1:L,2:R"),
            ..sound_from(None, dir.path())
        });
        assert!(folded.contains("_20_"), "{folded}");
    }

    #[test]
    fn a_dated_folder_name_gives_back_its_date() {
        let undated = "MyFilm_TST-1_F_EN-XX_20_2K_PPF_SMPTE_OV";
        assert_eq!(
            date_apart_from("MyFilm_TST-1_F_EN-XX_20_2K_20260816_PPF_SMPTE_OV", undated),
            Some(DATE)
        );
        assert_eq!(
            date_apart_from("MyFilm_TST-1_F_EN-XX_51_2K_20260816_PPF_SMPTE_OV", undated),
            None,
            "another channel field is another package"
        );
        assert_eq!(date_apart_from(undated, undated), None);
    }

    #[test]
    fn the_undated_name_is_the_name_without_its_date() {
        let config = config();
        let options = options();
        let naming = IsdcfNaming {
            config: &config,
            options: &options,
            sound: sound_from(None, Path::new("no-picture")),
            burnt_in_subtitle: false,
        };
        let dated = naming.name().unwrap();
        let undated = naming.name_with_date(None).unwrap();
        assert_eq!(date_apart_from(&dated, &undated), Some(DATE));
    }
}
