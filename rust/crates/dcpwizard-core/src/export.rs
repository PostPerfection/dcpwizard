use asdcplib::crypto::{AesDecContext, HmacContext};
use asdcplib::{EssenceType, jp2k, pcm};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

/// Export format for transcoding DCP content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExportFormat {
    ProRes,
    #[default]
    H264,
    H265,
    DnxHr,
    ImageSequence,
}

/// The formats that come out as one movie file. An image sequence has no
/// container, no pixel format of its own and no sound track, so it answers none
/// of these and is written as numbered PNGs instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MovieFormat {
    ProRes,
    H264,
    H265,
    DnxHr,
}

impl ExportFormat {
    fn movie(self) -> Option<MovieFormat> {
        match self {
            ExportFormat::ProRes => Some(MovieFormat::ProRes),
            ExportFormat::H264 => Some(MovieFormat::H264),
            ExportFormat::H265 => Some(MovieFormat::H265),
            ExportFormat::DnxHr => Some(MovieFormat::DnxHr),
            ExportFormat::ImageSequence => None,
        }
    }
}

impl MovieFormat {
    fn ffmpeg_codec(self) -> &'static str {
        match self {
            MovieFormat::ProRes => "prores_ks",
            MovieFormat::H264 => "libx264",
            MovieFormat::H265 => "libx265",
            MovieFormat::DnxHr => "dnxhd",
        }
    }

    fn file_extension(self) -> &'static str {
        match self {
            MovieFormat::ProRes => "mov",
            MovieFormat::H264 | MovieFormat::H265 => "mp4",
            MovieFormat::DnxHr => "mxf",
        }
    }

    fn pixel_format(self) -> &'static str {
        match self {
            MovieFormat::ProRes => "yuv422p10le",
            MovieFormat::DnxHr => "yuv422p",
            MovieFormat::H264 | MovieFormat::H265 => "yuv420p",
        }
    }

    // a ProRes or DNxHR master for approval carries PCM, only the delivery codecs take AAC
    fn audio_codec(self) -> &'static str {
        match self {
            MovieFormat::ProRes | MovieFormat::DnxHr => "pcm_s24le",
            MovieFormat::H264 | MovieFormat::H265 => "aac",
        }
    }
}

/// Export configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportConfig {
    pub input: PathBuf,
    pub output_path: PathBuf,
    pub format: ExportFormat,
    pub quality_crf: u32,
    pub audio_mxf: Option<PathBuf>,
    pub kdm: Option<PathBuf>,
    pub recipient_key: Option<PathBuf>,
    pub keys: Option<PathBuf>,
}

pub const CANCELLED: &str = "Cancelled";

const DEFAULT_CRF: u32 = 18;
const CPL_EXTENSION: &str = "xml";
// a frame waiting on a slow one is held in memory until it can be written
const FRAMES_IN_FLIGHT_PER_DECODE_WORKER: usize = 2;
const MICROSECONDS_PER_SECOND: u128 = 1_000_000;
const SOUND_WORK_PREFIX: &str = ".dcpwizard-export-";
const IMAGE_SEQUENCE_PATTERN: &str = "frame_%08d.png";
// ffmpeg numbers an image sequence from 1
const IMAGE_SEQUENCE_FIRST_NUMBER: u32 = 1;

fn image_sequence_frame_name(number: u32) -> String {
    format!("frame_{number:08}.png")
}

struct ExportReel {
    picture: PathBuf,
    sound: Option<PathBuf>,
    span: Option<ReelSpan>,
}

struct ReelSpan {
    cpl: PathBuf,
    entry_point: u64,
    sound_entry_point: u64,
    duration: u64,
}

impl ReelSpan {
    fn frames_within(&self, mxf: &Path, mxf_frames: u32) -> Result<Range<u32>, String> {
        let mxf_frames = u64::from(mxf_frames);
        // a reel with no Duration plays the rest of the asset
        let duration = if self.duration == 0 {
            mxf_frames.saturating_sub(self.entry_point)
        } else {
            self.duration
        };
        let end = self.entry_point + duration;
        if duration == 0 || end > mxf_frames {
            return Err(format!(
                "{} plays frames {}..{end} of {}, which holds {mxf_frames}",
                self.cpl.display(),
                self.entry_point,
                mxf.display()
            ));
        }
        Ok(self.entry_point as u32..end as u32)
    }
}

fn resolve_sources(config: &ExportConfig) -> Result<Vec<ExportReel>, String> {
    let input = &config.input;
    if !input.exists() {
        return Err(format!("input not found: {}", input.display()));
    }
    let cpl = if input.is_dir() {
        Some(only_cpl(input)?)
    } else if is_cpl_path(input) {
        Some(input.clone())
    } else {
        None
    };
    let Some(cpl) = cpl else {
        return Ok(vec![ExportReel {
            picture: input.clone(),
            sound: config.audio_mxf.clone(),
            span: None,
        }]);
    };
    let reels = crate::multi_cpl::get_timeline(&cpl);
    if reels.is_empty() {
        return Err(format!("{} has no reels", cpl.display()));
    }
    if reels.len() > 1 && config.audio_mxf.is_some() {
        return Err("a sound override takes a picture MXF or a single-reel CPL".to_string());
    }
    reels
        .into_iter()
        .enumerate()
        .map(|(index, reel)| {
            if reel.picture_file.is_empty() {
                return Err(format!(
                    "{} reel {} names a picture asset the ASSETMAP does not list",
                    cpl.display(),
                    index + 1
                ));
            }
            let cpl_sound = (!reel.sound_file.is_empty()).then(|| PathBuf::from(&reel.sound_file));
            Ok(ExportReel {
                picture: PathBuf::from(&reel.picture_file),
                sound: config.audio_mxf.clone().or(cpl_sound),
                span: Some(ReelSpan {
                    cpl: cpl.clone(),
                    entry_point: reel.entry_point,
                    sound_entry_point: reel.sound_entry_point,
                    duration: reel.duration_frames,
                }),
            })
        })
        .collect()
}

fn every_reel_sound(reels: &[ExportReel]) -> Result<Vec<PathBuf>, String> {
    let first_has_sound = reels[0].sound.is_some();
    if let Some(index) = reels
        .iter()
        .position(|reel| reel.sound.is_some() != first_has_sound)
    {
        let (without, with) = if first_has_sound {
            (index + 1, 1)
        } else {
            (1, index + 1)
        };
        return Err(format!(
            "reel {without} has no sound track while reel {with} does, \
             export takes a composition with sound on every reel or on none"
        ));
    }
    Ok(reels.iter().filter_map(|reel| reel.sound.clone()).collect())
}

fn is_cpl_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(CPL_EXTENSION))
}

fn only_cpl(directory: &Path) -> Result<PathBuf, String> {
    let cpls = crate::multi_cpl::list_cpls(directory);
    match cpls.as_slice() {
        [only] => Ok(directory.join(&only.file_path)),
        [] => Err(format!(
            "{} holds no CPL, export takes a picture MXF, a DCP directory or a CPL",
            directory.display()
        )),
        several => {
            let names: Vec<&str> = several.iter().map(|cpl| cpl.file_path.as_str()).collect();
            Err(format!(
                "{} holds {} CPLs ({}), pass the CPL path as --input",
                directory.display(),
                several.len(),
                names.join(", ")
            ))
        }
    }
}

struct PictureSource {
    reader: jp2k::MxfReader,
    width: u32,
    height: u32,
    edit_rate_num: i32,
    edit_rate_den: i32,
    frames: u32,
    writer_info: asdcplib::WriterInfo,
}

impl PictureSource {
    fn raster(&self) -> (u32, u32, i32, i32) {
        (
            self.width,
            self.height,
            self.edit_rate_num,
            self.edit_rate_den,
        )
    }

    fn describe_raster(&self) -> String {
        format!(
            "{}x{} at {}/{}",
            self.width, self.height, self.edit_rate_num, self.edit_rate_den
        )
    }
}

fn open_picture(mxf: &Path) -> Result<PictureSource, String> {
    let name = mxf.to_string_lossy();
    match asdcplib::essence_type(&name) {
        Ok(EssenceType::Jpeg2000) => {}
        Ok(EssenceType::Jpeg2000Stereo) => {
            return Err(format!(
                "{} is a stereoscopic picture MXF, export takes a 2D picture",
                mxf.display()
            ));
        }
        _ => return Err(format!("{} is not a JPEG 2000 picture MXF", mxf.display())),
    }
    let mxf_error = |error: asdcplib::Error| format!("{}: {error}", mxf.display());
    let mut reader = jp2k::MxfReader::new();
    reader.open_read(&name).map_err(mxf_error)?;
    let descriptor = reader.picture_descriptor().map_err(mxf_error)?;
    let writer_info = reader.writer_info().map_err(mxf_error)?;
    let rate = descriptor.edit_rate;
    if rate.numerator <= 0 || rate.denominator <= 0 {
        return Err(format!(
            "{} declares the edit rate {}/{}",
            mxf.display(),
            rate.numerator,
            rate.denominator
        ));
    }
    if descriptor.container_duration == 0 {
        return Err(format!("{} has no frames", mxf.display()));
    }
    Ok(PictureSource {
        reader,
        width: descriptor.stored_width,
        height: descriptor.stored_height,
        edit_rate_num: rate.numerator,
        edit_rate_den: rate.denominator,
        frames: descriptor.container_duration,
        writer_info,
    })
}

fn encrypted_sound_asset_id(sound: &Path) -> Result<Option<String>, String> {
    if !sound.exists() {
        return Err(format!("sound not found: {}", sound.display()));
    }
    let name = sound.to_string_lossy();
    let pcm_mxf = matches!(
        asdcplib::essence_type(&name),
        Ok(EssenceType::Pcm24b48k | EssenceType::Pcm24b96k)
    );
    if !pcm_mxf {
        return Ok(None);
    }
    let mxf_error = |error: asdcplib::Error| format!("{}: {error}", sound.display());
    let mut reader = pcm::MxfReader::new();
    reader.open_read(&name).map_err(mxf_error)?;
    let info = reader.writer_info().map_err(mxf_error)?;
    Ok(info
        .encrypted_essence
        .then(|| uuid::Uuid::from_bytes(info.asset_uuid).to_string()))
}

fn encrypted_without_keys(mxf: &Path) -> String {
    format!(
        "{} is encrypted, pass --kdm with --recipient-key or --keys",
        mxf.display()
    )
}

enum ExportTarget {
    Movie(PathBuf),
    ImageSequence {
        directory: PathBuf,
        created_directory: bool,
    },
}

impl ExportTarget {
    fn describe(&self) -> String {
        match self {
            ExportTarget::Movie(file) => format!("export to {}", file.display()),
            ExportTarget::ImageSequence { directory, .. } => {
                format!("write frames to {}", directory.display())
            }
        }
    }

    fn remove_partial(&self, frames: u32) {
        match self {
            ExportTarget::Movie(file) => {
                let _ = std::fs::remove_file(file);
            }
            ExportTarget::ImageSequence {
                directory,
                created_directory: true,
            } => {
                let _ = std::fs::remove_dir_all(directory);
            }
            ExportTarget::ImageSequence { directory, .. } => {
                for number in IMAGE_SEQUENCE_FIRST_NUMBER..IMAGE_SEQUENCE_FIRST_NUMBER + frames {
                    let _ = std::fs::remove_file(directory.join(image_sequence_frame_name(number)));
                }
            }
        }
    }
}

pub fn export_dcp(
    config: &ExportConfig,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    let reels = resolve_sources(config)?;
    let mut pictures: Vec<(PictureSource, Range<u32>)> = Vec::with_capacity(reels.len());
    for (index, reel) in reels.iter().enumerate() {
        if !reel.picture.exists() {
            return Err(format!("picture MXF not found: {}", reel.picture.display()));
        }
        let picture = open_picture(&reel.picture)?;
        if let Some((first, _)) = pictures.first()
            && picture.raster() != first.raster()
        {
            return Err(format!(
                "reel {} picture is {}, reel 1 is {}",
                index + 1,
                picture.describe_raster(),
                first.describe_raster()
            ));
        }
        let frames = match &reel.span {
            Some(span) => span.frames_within(&reel.picture, picture.frames)?,
            None => 0..picture.frames,
        };
        pictures.push((picture, frames));
    }
    let (width, height, edit_rate_num, edit_rate_den) = pictures[0].0.raster();

    let movie = config.format.movie();
    let sounds = match movie {
        Some(_) => every_reel_sound(&reels)?,
        None => Vec::new(),
    };
    let sounds = sounds
        .into_iter()
        .map(|sound| encrypted_sound_asset_id(&sound).map(|asset_id| (sound, asset_id)))
        .collect::<Result<Vec<_>, String>>()?;

    let key_source = postkit::content_keys::ContentKeys::from_options(
        config.kdm.as_deref(),
        config.recipient_key.as_deref(),
        config.keys.as_deref(),
    )?;
    let mut reel_pictures = Vec::with_capacity(pictures.len());
    for ((picture, frames), reel) in pictures.into_iter().zip(&reels) {
        let contexts = match (&key_source, picture.writer_info.encrypted_essence) {
            (_, false) => None,
            (Some(keys), true) => {
                Some(keys.decrypt_and_hmac_contexts(&picture.writer_info, "picture")?)
            }
            (None, true) => return Err(encrypted_without_keys(&reel.picture)),
        };
        reel_pictures.push(ReelPicture {
            reader: picture.reader,
            contexts,
            frames,
        });
    }
    let encrypted_sound = sounds.iter().find(|(_, asset_id)| asset_id.is_some());
    if let (Some((sound, _)), None) = (encrypted_sound, &key_source) {
        return Err(encrypted_without_keys(sound));
    }

    let crf = if config.quality_crf == 0 {
        DEFAULT_CRF
    } else {
        config.quality_crf
    };

    let mut command = std::process::Command::new("ffmpeg");
    command
        .args(["-y", "-v", "error", "-f", "rawvideo", "-pix_fmt", "xyz12le"])
        .arg("-s")
        .arg(format!("{width}x{height}"))
        .arg("-r")
        .arg(format!("{edit_rate_num}/{edit_rate_den}"))
        .args(["-i", "pipe:0"]);

    // removed on drop, on every path out
    let mut sound_work: Option<tempfile::TempDir> = None;
    let target = match movie {
        Some(movie) => {
            let output = if config.output_path.extension().is_none() {
                config.output_path.with_extension(movie.file_extension())
            } else {
                config.output_path.clone()
            };
            if encrypted_sound.is_some() {
                sound_work = Some(sound_work_directory(&output)?);
            }
            let fps = (f64::from(edit_rate_num) / f64::from(edit_rate_den)).round() as u32;
            let sound_inputs = sounds
                .into_iter()
                .map(
                    |(sound, asset_id)| match (asset_id, &key_source, &sound_work) {
                        (Some(asset_id), Some(keys), Some(work)) => {
                            let decrypted = crate::decrypt::process_sound(
                                &sound.to_string_lossy(),
                                &asset_id,
                                keys,
                                fps,
                                work.path(),
                            )?
                            .expect("a sound path and asset id give a sound track");
                            Ok(work.path().join(decrypted.filename))
                        }
                        _ => Ok(sound),
                    },
                )
                .collect::<Result<Vec<_>, String>>()?;
            let seconds = |frames: u64| exact_seconds(frames, edit_rate_num, edit_rate_den);
            for ((sound, reel), reel_picture) in sound_inputs.iter().zip(&reels).zip(&reel_pictures)
            {
                if let Some(span) = &reel.span {
                    command.arg("-ss").arg(seconds(span.sound_entry_point));
                    command
                        .arg("-t")
                        .arg(seconds(reel_picture.frames.len() as u64));
                }
                command.arg("-i").arg(sound);
            }
            if sound_inputs.len() > 1 {
                add_sound_concatenation(&mut command, sound_inputs.len());
            }
            add_movie_arguments(&mut command, movie, crf);
            command.arg(&output);
            ExportTarget::Movie(output)
        }
        None => {
            let directory = config.output_path.clone();
            let created_directory = !directory.exists();
            std::fs::create_dir_all(&directory)
                .map_err(|e| format!("could not create {}: {e}", directory.display()))?;
            command.arg(directory.join(IMAGE_SEQUENCE_PATTERN));
            ExportTarget::ImageSequence {
                directory,
                created_directory,
            }
        }
    };
    drop(key_source);

    let picture = ExportPicture {
        reels: reel_pictures,
        width,
        height,
    };
    let exported = run_export(command, picture, &target, cancel, on_progress);
    drop(sound_work);
    exported?;
    match &target {
        ExportTarget::Movie(output) => tracing::info!("Exported DCP to {}", output.display()),
        ExportTarget::ImageSequence { directory, .. } => {
            tracing::info!("Exported image sequence to {}", directory.display())
        }
    }
    Ok(())
}

// the picture is input 0, each reel's sound follows in reel order
fn add_sound_concatenation(command: &mut std::process::Command, sound_inputs: usize) {
    let inputs: String = (1..=sound_inputs)
        .map(|input| format!("[{input}:a]"))
        .collect();
    command
        .arg("-filter_complex")
        .arg(format!("{inputs}concat=n={sound_inputs}:v=0:a=1[sound]"));
    command.args(["-map", "0:v", "-map", "[sound]"]);
}

// ffmpeg keeps time in microseconds
fn exact_seconds(frames: u64, edit_rate_num: i32, edit_rate_den: i32) -> String {
    let microseconds = u128::from(frames) * edit_rate_den as u128 * MICROSECONDS_PER_SECOND
        / edit_rate_num as u128;
    format!(
        "{}.{:06}",
        microseconds / MICROSECONDS_PER_SECOND,
        microseconds % MICROSECONDS_PER_SECOND
    )
}

fn sound_work_directory(output: &Path) -> Result<tempfile::TempDir, String> {
    let parent = match output.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    tempfile::Builder::new()
        .prefix(SOUND_WORK_PREFIX)
        .tempdir_in(parent)
        .map_err(|e| {
            format!(
                "could not create a work directory in {}: {e}",
                parent.display()
            )
        })
}

fn add_movie_arguments(command: &mut std::process::Command, movie: MovieFormat, crf: u32) {
    command.arg("-vf").arg(rec709_filter(movie));
    command.arg("-c:v").arg(movie.ffmpeg_codec());

    match movie {
        MovieFormat::H264 | MovieFormat::H265 => {
            command.arg("-crf").arg(crf.to_string());
            command.arg("-preset").arg("medium");
        }
        MovieFormat::ProRes => {
            command.arg("-profile:v").arg("3"); // ProRes HQ
        }
        MovieFormat::DnxHr => {
            command.arg("-profile:v").arg("dnxhr_hq");
        }
    }

    command.arg("-c:a").arg(movie.audio_codec());
}

// a DCP picture is X'Y'Z' at DCI gamma 2.6: swscale undoes that, out_color_matrix picks the
// Rec.709 matrix over swscale's 601 default, and setparams tags what the player has to assume
fn rec709_filter(movie: MovieFormat) -> String {
    format!(
        "scale=out_color_matrix=bt709:out_range=tv,format={},\
         setparams=color_primaries=bt709:color_trc=bt709:colorspace=bt709:range=tv",
        movie.pixel_format()
    )
}

type PictureContexts = (AesDecContext, HmacContext);

struct ReelPicture {
    reader: jp2k::MxfReader,
    contexts: Option<PictureContexts>,
    frames: Range<u32>,
}

struct ExportPicture {
    reels: Vec<ReelPicture>,
    width: u32,
    height: u32,
}

impl ExportPicture {
    fn frames(&self) -> u32 {
        self.reels.iter().map(|reel| reel.frames.len() as u32).sum()
    }
}

struct ReadCodestream {
    position: u32,
    frame_index: u32,
    codestream: Result<Vec<u8>, String>,
}

struct DecodedPicture {
    position: u32,
    picture: Result<Vec<u8>, String>,
}

enum PipelineStop {
    Cancelled,
    Failed(String),
    FfmpegClosedInput,
}

fn run_export(
    mut command: std::process::Command,
    picture: ExportPicture,
    target: &ExportTarget,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let child = command
        .spawn()
        .map_err(|e| format!("could not run ffmpeg: {e}"))?;
    let frames = picture.frames();
    let exported = drive_ffmpeg(child, picture, target, cancel, on_progress);
    // the output path may hold the user's own file until ffmpeg has started
    if exported.is_err() {
        target.remove_partial(frames);
    }
    exported
}

fn drive_ffmpeg(
    mut child: Child,
    picture: ExportPicture,
    target: &ExportTarget,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    let stdin = child.stdin.take().expect("ffmpeg's stdin is piped");
    let mut stderr = child.stderr.take().expect("ffmpeg's stderr is piped");
    // a full stderr pipe would stop ffmpeg reading frames
    let stderr_text = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let every_frame_written = match stream_frames(picture, stdin, cancel, on_progress) {
        Ok(()) => true,
        // a terminal's Ctrl+C reaches ffmpeg too
        Err(PipelineStop::FfmpegClosedInput) if cancel.load(Ordering::Relaxed) => {
            return Err(stop_ffmpeg(child, CANCELLED.to_string()));
        }
        Err(PipelineStop::FfmpegClosedInput) => false,
        Err(PipelineStop::Cancelled) => return Err(stop_ffmpeg(child, CANCELLED.to_string())),
        Err(PipelineStop::Failed(message)) => return Err(stop_ffmpeg(child, message)),
    };

    let status = child
        .wait()
        .map_err(|e| format!("could not wait for ffmpeg: {e}"))?;
    let stderr_text = stderr_text
        .join()
        .expect("the stderr reader does not panic");
    if status.success() && every_frame_written {
        return Ok(());
    }
    Err(format!(
        "ffmpeg could not {}: {}",
        target.describe(),
        stderr_text.trim()
    ))
}

// killed so ffmpeg does not finish a movie from the frames it has
fn stop_ffmpeg(mut child: Child, message: String) -> String {
    let _ = child.kill();
    let _ = child.wait();
    message
}

fn stream_frames(
    picture: ExportPicture,
    stdin: ChildStdin,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<(), PipelineStop> {
    let frames = picture.frames();
    let ExportPicture {
        reels,
        width,
        height,
    } = picture;
    let workers = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
    let in_flight = workers * FRAMES_IN_FLIGHT_PER_DECODE_WORKER;

    let (codestream_sender, codestream_receiver) = sync_channel::<ReadCodestream>(in_flight);
    let (decoded_sender, decoded_receiver) = sync_channel::<DecodedPicture>(in_flight);
    let (slot_sender, slot_receiver) = sync_channel::<()>(in_flight);
    for _ in 0..in_flight {
        slot_sender
            .send(())
            .expect("the slot channel holds one slot per frame in flight");
    }
    let codestream_receiver = Mutex::new(codestream_receiver);
    let stopped = AtomicBool::new(false);

    std::thread::scope(|scope| {
        let stopped = &stopped;
        let codestream_receiver = &codestream_receiver;
        scope.spawn(move || {
            read_codestreams(reels, codestream_sender, slot_receiver, cancel, stopped)
        });
        for _ in 0..workers {
            let decoded_sender = decoded_sender.clone();
            scope.spawn(move || {
                decode_codestreams(codestream_receiver, decoded_sender, width, height)
            });
        }
        drop(decoded_sender);
        let written = write_in_order(
            decoded_receiver,
            slot_sender,
            stdin,
            frames,
            cancel,
            on_progress,
        );
        stopped.store(true, Ordering::Relaxed);
        written
    })
}

fn read_codestreams(
    reels: Vec<ReelPicture>,
    codestreams: SyncSender<ReadCodestream>,
    slots: Receiver<()>,
    cancel: &AtomicBool,
    stopped: &AtomicBool,
) {
    let mut buffer = vec![0u8; crate::decrypt::MAX_FRAME_BUF];
    let mut position = 0u32;
    for reel in reels {
        let ReelPicture {
            mut reader,
            mut contexts,
            frames,
        } = reel;
        for index in frames {
            if cancel.load(Ordering::Relaxed) || stopped.load(Ordering::Relaxed) {
                return;
            }
            if slots.recv().is_err() {
                return;
            }
            let read = match &mut contexts {
                Some((decrypt, integrity)) => reader
                    .read_frame(index, &mut buffer, Some(decrypt), Some(integrity))
                    .map_err(|e| {
                        format!("decrypt picture frame {index} (wrong key or MIC mismatch): {e}")
                    }),
                None => reader
                    .read_frame(index, &mut buffer, None, None)
                    .map_err(|e| format!("cannot read picture frame {index}: {e}")),
            };
            let failed = read.is_err();
            let codestream = read.map(|size| buffer[..size].to_vec());
            let read_codestream = ReadCodestream {
                position,
                frame_index: index,
                codestream,
            };
            if codestreams.send(read_codestream).is_err() || failed {
                return;
            }
            position += 1;
        }
    }
}

fn decode_codestreams(
    codestreams: &Mutex<Receiver<ReadCodestream>>,
    decoded: SyncSender<DecodedPicture>,
    width: u32,
    height: u32,
) {
    loop {
        let Ok(read) = codestreams
            .lock()
            .expect("no decode worker panics holding the codestream queue")
            .recv()
        else {
            return;
        };
        let picture = read
            .codestream
            .and_then(|codestream| decode_to_xyz12le(read.frame_index, codestream, width, height));
        let decoded_picture = DecodedPicture {
            position: read.position,
            picture,
        };
        if decoded.send(decoded_picture).is_err() {
            return;
        }
    }
}

fn decode_to_xyz12le(
    index: u32,
    codestream: Vec<u8>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let frame = postkit::grok_decoder::decode(codestream, 0)
        .map_err(|e| format!("cannot decode picture frame {index}: {e}"))?;
    if (frame.width, frame.height) != (width, height) {
        return Err(format!(
            "picture frame {index} decodes to {}x{}, the picture descriptor declares {width}x{height}",
            frame.width, frame.height
        ));
    }
    frame
        .to_xyz12le()
        .map_err(|e| format!("picture frame {index}: {e}"))
}

fn write_in_order(
    decoded: Receiver<DecodedPicture>,
    slots: SyncSender<()>,
    mut stdin: ChildStdin,
    frames: u32,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<(), PipelineStop> {
    let mut waiting = BTreeMap::new();
    let mut next = 0u32;
    while next < frames {
        if cancel.load(Ordering::Relaxed) {
            return Err(PipelineStop::Cancelled);
        }
        let Ok(decoded_picture) = decoded.recv() else {
            if cancel.load(Ordering::Relaxed) {
                return Err(PipelineStop::Cancelled);
            }
            return Err(PipelineStop::Failed(format!(
                "the picture reader stopped before frame {next}"
            )));
        };
        waiting.insert(
            decoded_picture.position,
            decoded_picture.picture.map_err(PipelineStop::Failed)?,
        );
        while let Some(picture) = waiting.remove(&next) {
            if stdin.write_all(&picture).is_err() {
                return Err(PipelineStop::FfmpegClosedInput);
            }
            next += 1;
            on_progress(u64::from(next), u64::from(frames));
            // fails once the reader has read the last frame
            let _ = slots.send(());
        }
    }
    Ok(())
}

/// Extract a single frame from an MXF at the given frame number.
pub fn extract_frame(mxf_path: &Path, frame_number: u64, output_path: &Path) -> i32 {
    let fps = 24; // Default DCP frame rate
    let timestamp = format!(
        "{:02}:{:02}:{:02}.{:03}",
        frame_number / (fps * 3600),
        (frame_number / (fps * 60)) % 60,
        (frame_number / fps) % 60,
        ((frame_number % fps) * 1000) / fps
    );

    let result = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-ss")
        .arg(&timestamp)
        .arg("-i")
        .arg(mxf_path)
        .arg("-frames:v")
        .arg("1")
        .arg(output_path)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            tracing::info!(
                "Extracted frame {} to {}",
                frame_number,
                output_path.display()
            );
            0
        }
        Ok(o) => {
            tracing::error!(
                "ffmpeg frame extraction failed: {}",
                String::from_utf8_lossy(&o.stderr)
            );
            -1
        }
        Err(e) => {
            tracing::error!("Failed to run ffmpeg: {e}");
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PICTURE_MXF_FRAMES: usize = 4;
    const PICTURE_MXF_FPS: u32 = 24;

    fn write_picture_mxf(directory: &Path) -> PathBuf {
        let codestream = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../extern/postkit/tests/fixtures/cinema2k_64x64.j2c"),
        )
        .unwrap();
        let input_files = (0..PICTURE_MXF_FRAMES)
            .map(|frame| {
                let path = directory.join(format!("frame_{frame}.j2c"));
                std::fs::write(&path, &codestream).unwrap();
                path
            })
            .collect();
        let mxf = directory.join("picture.mxf");
        let track = postkit::mxf_wrap::mxf_wrap(&postkit::mxf_wrap::MxfWrapOptions {
            input_files,
            output: mxf.clone(),
            essence_type: postkit::mxf_wrap::EssenceType::J2k,
            standard: postkit::mxf_wrap::MxfStandard::AsDcp,
            fps_num: PICTURE_MXF_FPS,
            fps_den: 1,
            partition_size: 0,
            encryption: None,
            mca_config: None,
            resource_ids: Vec::new(),
            hdr: None,
            asset_uuid: None,
            timed_text_duration_frames: None,
        });
        assert!(track.success, "wrap failed: {}", track.error);
        mxf
    }

    #[test]
    fn a_cancelled_export_returns_cancelled_and_leaves_no_output() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("screener.mov");
        let config = ExportConfig {
            input: write_picture_mxf(directory.path()),
            output_path: output.clone(),
            format: ExportFormat::ProRes,
            ..ExportConfig::default()
        };
        let mut reported = Vec::new();

        let result = export_dcp(&config, &AtomicBool::new(true), &mut |frame, total| {
            reported.push((frame, total))
        });

        assert_eq!(result.unwrap_err(), CANCELLED);
        assert!(
            !output.exists(),
            "the cancelled export left {}",
            output.display()
        );
        assert!(
            reported.is_empty(),
            "the cancelled export wrote {reported:?}"
        );
    }
}
