use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use postkit::encode::InputType;
use postkit::picture_processing::PictureProcessing;
use postkit::pipeline::{EncodeRunOptions, PipelineProgress};
use postkit::timeline::{Clip, ClipSound, SoundTrack, Timeline};

use crate::source_picture::{EncodeGeometry, SourcePictureOptions};

pub const TIMELINE_SOUND_FILE: &str = "timeline_sound.wav";
pub const TIMELINE_PULLED_UP_SOUND_FILE: &str = "timeline_sound_pullup.wav";
pub const TIMELINE_SUBTITLE_FILE: &str = "timeline_subtitles.srt";

pub enum TimelineShape {
    SingleClip(SingleClipBuild),
    MultiClip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SingleClipBuild {
    pub picture: PathBuf,
    pub trim_start_frames: u64,
    pub trim_end_frames: u64,
    pub audio: Option<PathBuf>,
    // the picture's own sound is not taken either
    pub silent: bool,
    pub subtitle: Option<PathBuf>,
    // the subtitle is timed against the clip's source
    pub subtitle_from_clip: bool,
    pub reel_split_frames: Vec<u64>,
}

pub fn shape(timeline: &Timeline) -> Result<TimelineShape, String> {
    let [clip] = timeline.clips.as_slice() else {
        return Ok(TimelineShape::MultiClip);
    };
    timeline.check()?;
    let name = clip_name(0, clip);
    if clip.source_colour != postkit::encode::SourceColour::default() {
        return Err(format!(
            "{name} carries its own source colour, and a one-clip timeline takes the colour from \
             --source-colourspace"
        ));
    }
    if !clip.picture_processing.is_identity() {
        return Err(format!(
            "{name} carries its own picture processing, and a one-clip timeline takes it from the \
             picture flags"
        ));
    }
    refuse_two_subtitle_sources(timeline)?;
    let source_frames = source_frame_count(&clip.picture).map_err(|e| format!("{name}: {e}"))?;
    let audio = match (&timeline.sound, &clip.sound) {
        (SoundTrack::Override(path), _) | (SoundTrack::PerClip, ClipSound::File(path)) => {
            Some(path.clone())
        }
        (SoundTrack::PerClip, _) => None,
    };
    Ok(TimelineShape::SingleClip(SingleClipBuild {
        picture: clip.picture.clone(),
        trim_start_frames: clip.in_frame,
        trim_end_frames: source_frames - clip.out_frame,
        audio,
        silent: timeline.sound == SoundTrack::PerClip && clip.sound == ClipSound::None,
        subtitle: clip.subtitle.clone().or_else(|| timeline.subtitle.clone()),
        subtitle_from_clip: clip.subtitle.is_some(),
        reel_split_frames: timeline.reel_starts.clone(),
    }))
}

pub fn carries_subtitle(timeline: &Timeline) -> bool {
    timeline.subtitle.is_some() || timeline.clips.iter().any(|clip| clip.subtitle.is_some())
}

fn refuse_two_subtitle_sources(timeline: &Timeline) -> Result<(), String> {
    let Some((index, clip)) = timeline
        .clips
        .iter()
        .enumerate()
        .find(|(_, clip)| clip.subtitle.is_some())
    else {
        return Ok(());
    };
    match &timeline.subtitle {
        Some(subtitle) => Err(format!(
            "the timeline's subtitle {} and the subtitle of {} both time the subtitle track: keep \
             one",
            subtitle.display(),
            clip_name(index, clip)
        )),
        None => Ok(()),
    }
}

fn clip_name(index: usize, clip: &Clip) -> String {
    format!("clip {} ({})", index + 1, clip.picture.display())
}

// counted the way the create command counts the picture it trims
fn source_frame_count(picture: &Path) -> Result<u64, String> {
    match postkit::encode::detect_input_type(picture) {
        InputType::J2kSequence => Ok(crate::trim::frame_count(picture)),
        InputType::ImageSequence if picture.is_dir() => {
            postkit::encode::find_source_frames(picture)
                .map(|frames| frames.len() as u64)
                .map_err(|e| format!("cannot list {}: {e}", picture.display()))
        }
        InputType::Video | InputType::PictureMxf => crate::probe::probe_video(picture)
            .map(|info| u64::from(info.total_frames))
            .ok_or_else(|| "ffprobe cannot read it".to_string()),
        _ => Err(
            "a one-clip timeline takes a video, a picture MXF, or a directory of images or \
             codestreams"
                .to_string(),
        ),
    }
}

// each field is the option's name as the front end shows it, None when it is not set
#[derive(Debug, Default, Clone, Copy)]
pub struct MultiClipFlags {
    pub burn_subtitle: Option<&'static str>,
    pub resume: Option<&'static str>,
    pub right_eye: Option<&'static str>,
    pub hdr_dci: Option<&'static str>,
    pub allow_generic_hdr_tonemap: Option<&'static str>,
    pub input_range: Option<&'static str>,
    pub watermark: Option<&'static str>,
    pub still_length: Option<&'static str>,
    pub trim_start: Option<&'static str>,
    pub trim_end: Option<&'static str>,
    pub split_at: Option<&'static str>,
    pub split_chapters: Option<&'static str>,
    pub reel_length: Option<&'static str>,
    pub video_fade_in: Option<&'static str>,
    pub video_fade_out: Option<&'static str>,
}

pub const CLIPS_CARRY_THEIR_LENGTH: &str = "each clip carries its own in and out points";
pub const REEL_STARTS_COME_FROM_THE_TIMELINE: &str = "the reel starts come from the timeline";
const FADE_RESTARTS_ON_EVERY_CLIP: &str = "the fade would start again on every clip";

pub fn refuse_with_multi_clip(flags: &MultiClipFlags) -> Vec<String> {
    [
        (flags.burn_subtitle, None),
        (flags.resume, None),
        (flags.right_eye, None),
        (flags.hdr_dci, None),
        (flags.allow_generic_hdr_tonemap, None),
        (flags.input_range, None),
        (flags.watermark, None),
        (flags.still_length, Some(CLIPS_CARRY_THEIR_LENGTH)),
        (flags.trim_start, Some(CLIPS_CARRY_THEIR_LENGTH)),
        (flags.trim_end, Some(CLIPS_CARRY_THEIR_LENGTH)),
        (flags.split_at, Some(REEL_STARTS_COME_FROM_THE_TIMELINE)),
        (
            flags.split_chapters,
            Some(REEL_STARTS_COME_FROM_THE_TIMELINE),
        ),
        (flags.reel_length, Some(REEL_STARTS_COME_FROM_THE_TIMELINE)),
        (flags.video_fade_in, Some(FADE_RESTARTS_ON_EVERY_CLIP)),
        (flags.video_fade_out, Some(FADE_RESTARTS_ON_EVERY_CLIP)),
    ]
    .into_iter()
    .filter_map(|(name, reason)| Some((name?, reason)))
    .map(|(name, reason)| match reason {
        Some(reason) => {
            format!("{name} is not supported on a timeline of more than one clip: {reason}")
        }
        None => format!("{name} is not supported on a timeline of more than one clip"),
    })
    .collect()
}

// one raster for every clip: the first clip's unless a flag fixed it
pub struct CompositionPicture {
    pub raster: (u32, u32),
    geometry: EncodeGeometry,
    options: SourcePictureOptions,
}

impl CompositionPicture {
    pub fn resolve(
        timeline: &Timeline,
        options: &SourcePictureOptions,
        geometry: &EncodeGeometry,
    ) -> Result<CompositionPicture, String> {
        let first_clip = timeline
            .clips
            .first()
            .ok_or_else(|| "the timeline has no clips".to_string())?;
        let raster = match geometry.forced_raster {
            Some(raster) => raster,
            None => clip_raster(first_clip, options, geometry)
                .map_err(|e| format!("{}: {e}", clip_name(0, first_clip)))?,
        };
        Ok(CompositionPicture {
            raster,
            geometry: EncodeGeometry {
                forced_raster: Some(raster),
                container: geometry.container.or(Some(raster)),
            },
            options: options.clone(),
        })
    }

    pub fn clip_processing(&self, clip: &Clip) -> Result<PictureProcessing, String> {
        let input_type = postkit::encode::detect_input_type(&clip.picture);
        if !crate::preflight::is_precompressed(input_type) {
            return Ok(resolve_clip(clip, &self.options, &self.geometry)?.processing);
        }
        let size = clip_source_size(&clip.picture)?;
        if size != self.raster {
            return Err(format!(
                "its codestreams are {}x{} and the timeline's raster is {}x{}: compressed picture \
                 cannot be fitted",
                size.0, size.1, self.raster.0, self.raster.1
            ));
        }
        Ok(PictureProcessing::default())
    }
}

fn clip_raster(
    clip: &Clip,
    options: &SourcePictureOptions,
    geometry: &EncodeGeometry,
) -> Result<(u32, u32), String> {
    let input_type = postkit::encode::detect_input_type(&clip.picture);
    if crate::preflight::is_precompressed(input_type) {
        return clip_source_size(&clip.picture);
    }
    let resolved = resolve_clip(clip, options, geometry)?;
    Ok((resolved.encode_width, resolved.encode_height))
}

fn resolve_clip(
    clip: &Clip,
    options: &SourcePictureOptions,
    geometry: &EncodeGeometry,
) -> Result<crate::source_picture::ResolvedPicture, String> {
    let (width, height) = clip_source_size(&clip.picture)?;
    crate::source_picture::resolve_picture(
        options,
        &clip.picture,
        width,
        height,
        geometry,
        postkit::encode::detect_input_type(&clip.picture) == InputType::ImageSequence,
    )
}

fn clip_source_size(picture: &Path) -> Result<(u32, u32), String> {
    let size = match postkit::encode::detect_input_type(picture) {
        InputType::ImageSequence if picture.is_dir() => crate::encode::first_image_raster(picture),
        InputType::J2kSequence => crate::reel::collect_frames(picture)
            .first()
            .map(|frame| crate::pad::read_j2k_dimensions(frame))
            .transpose()?,
        _ => crate::probe::probe_video(picture).map(|info| (info.width, info.height)),
    };
    size.ok_or_else(|| format!("cannot read the size of {}", picture.display()))
}

#[derive(Debug)]
pub struct MaterialisedTimeline {
    pub j2k_dir: PathBuf,
    pub audio: Option<PathBuf>,
    pub subtitle: Option<PathBuf>,
    pub frames: u64,
    pub picture_findings: postkit::picture_findings::PictureFindings,
}

#[allow(clippy::too_many_arguments)]
pub fn materialise(
    timeline: &Timeline,
    work_dir: &Path,
    options: &EncodeRunOptions,
    per_clip_picture: impl Fn(&Clip) -> Result<PictureProcessing, String>,
    cancel: &Arc<AtomicBool>,
    pause: &Arc<AtomicBool>,
    on_progress: impl Fn(&PipelineProgress),
    on_log: impl Fn(&str),
) -> Result<MaterialisedTimeline, String> {
    timeline.check()?;
    refuse_two_subtitle_sources(timeline)?;
    let mut fitted = timeline.clone();
    for (index, clip) in fitted.clips.iter_mut().enumerate() {
        if clip.picture_processing.is_identity() {
            clip.picture_processing =
                per_clip_picture(clip).map_err(|e| format!("{}: {e}", clip_name(index, clip)))?;
        }
    }
    std::fs::create_dir_all(work_dir)
        .map_err(|e| format!("cannot create {}: {e}", work_dir.display()))?;
    let encoded = postkit::pipeline::run_encode_timeline(
        &fitted,
        work_dir,
        options,
        cancel,
        pause,
        on_progress,
        on_log,
    )?;
    let rendered_sound =
        postkit::timeline::render_sound(timeline, &work_dir.join(TIMELINE_SOUND_FILE))?;
    // a picture read faster than its own rate needs its sound pulled up to match
    let audio = match (rendered_sound, options.read_source_at) {
        (Some(sound), Some(_)) => {
            let pulled_up = work_dir.join(TIMELINE_PULLED_UP_SOUND_FILE);
            crate::hfr::audio_pull_up(&sound, &pulled_up)?;
            Some(pulled_up)
        }
        (sound, None) => sound,
        (None, Some(_)) => None,
    };
    let subtitle = match postkit::timeline::join_subtitles(
        timeline,
        &work_dir.join(TIMELINE_SUBTITLE_FILE),
    )? {
        Some(joined) => Some(joined),
        None => timeline.subtitle.clone(),
    };
    Ok(MaterialisedTimeline {
        j2k_dir: encoded.j2k_dir,
        audio,
        subtitle,
        frames: encoded.frames_encoded,
        picture_findings: encoded.picture_findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use postkit::encode::FrameRate;

    const SOURCE_FRAMES: u64 = 10;

    fn codestream_directory(parent: &Path, name: &str) -> PathBuf {
        let directory = parent.join(name);
        std::fs::create_dir_all(&directory).unwrap();
        for frame in 0..SOURCE_FRAMES {
            std::fs::write(directory.join(format!("frame_{frame:08}.j2c")), [0u8]).unwrap();
        }
        directory
    }

    fn clip(picture: PathBuf, in_frame: u64, out_frame: u64) -> Clip {
        Clip {
            picture,
            in_frame,
            out_frame,
            sound: ClipSound::None,
            subtitle: None,
            source_colour: Default::default(),
            picture_processing: Default::default(),
        }
    }

    fn timeline(clips: Vec<Clip>) -> Timeline {
        Timeline {
            version: postkit::timeline::TIMELINE_FORMAT_VERSION,
            fps: FrameRate::whole(24),
            clips,
            sound: SoundTrack::PerClip,
            subtitle: None,
            reel_starts: Vec::new(),
        }
    }

    #[test]
    fn one_clip_maps_its_in_and_out_points_onto_the_trims() {
        let directory = tempfile::tempdir().unwrap();
        let picture = codestream_directory(directory.path(), "feature");
        let one_clip = timeline(vec![clip(picture.clone(), 2, 7)]);

        let TimelineShape::SingleClip(build) = shape(&one_clip).unwrap() else {
            panic!("one clip is today's one-picture build");
        };
        assert_eq!(build.picture, picture);
        assert_eq!(build.trim_start_frames, 2);
        assert_eq!(build.trim_end_frames, SOURCE_FRAMES - 7);
        assert!(build.silent);
        assert_eq!(build.audio, None);
    }

    #[test]
    fn a_clip_subtitle_beside_the_timeline_subtitle_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let picture = codestream_directory(directory.path(), "feature");
        let mut one_clip = timeline(vec![Clip {
            subtitle: Some(directory.path().join("clip.srt")),
            ..clip(picture, 0, SOURCE_FRAMES)
        }]);
        one_clip.subtitle = Some(directory.path().join("timeline.srt"));

        let Err(refusal) = shape(&one_clip) else {
            panic!("two subtitle sources have to be refused");
        };
        assert!(
            refusal.contains("timeline.srt") && refusal.contains("clip 1"),
            "{refusal}"
        );
    }

    #[test]
    fn the_multi_clip_refusals_name_each_flag_given() {
        let refusals = refuse_with_multi_clip(&MultiClipFlags {
            video_fade_in: Some("--video-fade-in"),
            resume: Some("--resume"),
            ..Default::default()
        });
        assert_eq!(refusals.len(), 2, "{refusals:?}");
        assert!(refusals.iter().any(|r| r.starts_with("--video-fade-in")));
        assert!(refusals.iter().any(|r| r.starts_with("--resume")));
    }
}
