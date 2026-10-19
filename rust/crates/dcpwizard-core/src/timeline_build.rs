use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use postkit::encode::InputType;
use postkit::picture_processing::{Fit, PictureProcessing, Placement};
use postkit::pipeline::{EncodeRunOptions, PipelineProgress};
use postkit::timeline::{Clip, Timeline};

use crate::source_picture::{EncodeGeometry, ResolvedPicture, SourcePictureOptions};

pub const TIMELINE_SOUND_FILE: &str = "timeline_sound.wav";
pub const TIMELINE_PULLED_UP_SOUND_FILE: &str = "timeline_sound_pullup.wav";
pub const TIMELINE_SUBTITLE_FILE: &str = "timeline_subtitles.srt";

pub fn carries_subtitle(timeline: &Timeline) -> bool {
    timeline.subtitle.is_some() || timeline.clips.iter().any(|clip| clip.subtitle.is_some())
}

fn clip_name(index: usize, clip: &Clip) -> String {
    format!("clip {} ({})", index + 1, clip.picture.display())
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

// the probe, the raster and the package name read this clip
pub fn first_base_clip(timeline: &Timeline) -> Result<(usize, &Clip), String> {
    timeline.base_clips().first().copied().ok_or_else(|| {
        "layer 0 of the timeline holds no clip, and the picture is sized from its first one"
            .to_string()
    })
}

// one raster for every clip: the first layer 0 clip's unless a flag fixed it
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
        let (first_index, first_clip) = first_base_clip(timeline)?;
        let raster = match geometry.forced_raster {
            Some(raster) => raster,
            None => clip_raster(first_clip, options, geometry)
                .map_err(|e| format!("{}: {e}", clip_name(first_index, first_clip)))?,
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
        Ok(self.resolved_clip_picture(clip)?.processing)
    }

    // postkit sizes the black in a layer 0 gap from the fit of the encode's own picture options
    fn gap_processing(&self) -> PictureProcessing {
        let (width, height) = self.raster;
        PictureProcessing {
            fit: Some(Fit {
                box_width: width,
                box_height: height,
                raster_width: width,
                raster_height: height,
                placement: Placement::default(),
            }),
            ..PictureProcessing::default()
        }
    }

    pub fn resolved_clip_picture(&self, clip: &Clip) -> Result<ResolvedPicture, String> {
        let input_type = postkit::encode::detect_input_type(&clip.picture);
        if !crate::preflight::is_precompressed(input_type) {
            return resolve_clip(clip, &self.options, &self.geometry);
        }
        let size = clip_source_size(&clip.picture)?;
        if size != self.raster {
            return Err(format!(
                "its codestreams are {}x{} and the timeline's raster is {}x{}: compressed picture \
                 cannot be fitted",
                size.0, size.1, self.raster.0, self.raster.1
            ));
        }
        let processing = PictureProcessing::default();
        let plan = processing.plan(size.0, size.1)?;
        Ok(ResolvedPicture {
            encode_width: plan.output_width,
            encode_height: plan.output_height,
            processing,
            plan,
        })
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
) -> Result<ResolvedPicture, String> {
    let (width, height) = clip_source_size(&clip.picture)?;
    crate::source_picture::resolve_picture(
        options,
        &clip.picture,
        width,
        height,
        geometry,
        crate::source_picture::is_image_sequence_directory(&clip.picture),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineSound {
    FromSoundTrack,
    // a channel set given beside the timeline is its sound, the way an override WAV is
    FromChannelSet,
}

#[allow(clippy::too_many_arguments)]
pub fn materialise(
    timeline: &Timeline,
    sound: TimelineSound,
    work_dir: &Path,
    options: &EncodeRunOptions,
    composition: &CompositionPicture,
    cancel: &Arc<AtomicBool>,
    pause: &Arc<AtomicBool>,
    on_progress: impl Fn(&PipelineProgress),
    on_log: impl Fn(&str),
) -> Result<MaterialisedTimeline, String> {
    timeline.check()?;
    postkit::timeline::refuse_two_subtitle_sources(timeline)?;
    let mut fitted = timeline.clone();
    for (index, clip) in fitted.clips.iter_mut().enumerate() {
        if clip.picture_processing.is_identity() {
            clip.picture_processing = composition
                .clip_processing(clip)
                .map_err(|e| format!("{}: {e}", clip_name(index, clip)))?;
        }
    }
    let options = if timeline.base_gaps().is_empty() {
        options.clone()
    } else {
        EncodeRunOptions {
            picture: composition.gap_processing(),
            ..options.clone()
        }
    };
    std::fs::create_dir_all(work_dir)
        .map_err(|e| format!("cannot create {}: {e}", work_dir.display()))?;
    let encoded = postkit::pipeline::run_encode_timeline(
        &fitted,
        work_dir,
        &options,
        cancel,
        pause,
        on_progress,
        on_log,
    )?;
    let rendered_sound = match sound {
        TimelineSound::FromSoundTrack => {
            postkit::timeline::render_sound(timeline, &work_dir.join(TIMELINE_SOUND_FILE))?
        }
        TimelineSound::FromChannelSet => None,
    };
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

    const CARD_WIDTH: u32 = 320;
    const CARD_HEIGHT: u32 = 240;
    const CARD_BAR_HEIGHT: u32 = 32;

    fn write_image(path: &Path, filter: &str) {
        let written = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg(format!("color=c=red:s={CARD_WIDTH}x{CARD_HEIGHT}"))
            .args(["-vf", filter, "-frames:v", "1"])
            .arg(path)
            .output()
            .expect("ffmpeg has to run");
        assert!(
            written.status.success(),
            "{}",
            String::from_utf8_lossy(&written.stderr)
        );
    }

    #[test]
    fn a_still_clip_is_auto_cropped_from_its_own_picture_not_its_folder() {
        let directory = tempfile::tempdir().unwrap();
        let card = directory.path().join("card.png");
        let picture_height = CARD_HEIGHT - 2 * CARD_BAR_HEIGHT;
        write_image(
            &card,
            &format!(
                "crop={CARD_WIDTH}:{picture_height},pad={CARD_WIDTH}:{CARD_HEIGHT}:0:\
                 {CARD_BAR_HEIGHT}:black"
            ),
        );
        write_image(&directory.path().join("no_borders.png"), "null");
        let timeline = Timeline {
            version: postkit::timeline::TIMELINE_FORMAT_VERSION,
            fps: postkit::encode::FrameRate::whole(24),
            clips: vec![Clip {
                picture: card,
                start: 0,
                layer: 0,
                opacity: 1.0,
                in_frame: 0,
                out_frame: 48,
                sound: postkit::timeline::ClipSound::None,
                subtitle: None,
                source_colour: Default::default(),
                picture_processing: Default::default(),
            }],
            sound: postkit::timeline::SoundTrack::PerClip,
            subtitle: None,
            reel_starts: Vec::new(),
        };

        let composition = CompositionPicture::resolve(
            &timeline,
            &SourcePictureOptions {
                auto_crop: true,
                ..SourcePictureOptions::default()
            },
            &EncodeGeometry::default(),
        )
        .unwrap();
        let crop = composition
            .clip_processing(&timeline.clips[0])
            .unwrap()
            .crop;

        assert_eq!((crop.top, crop.bottom), (CARD_BAR_HEIGHT, CARD_BAR_HEIGHT));
    }
}
