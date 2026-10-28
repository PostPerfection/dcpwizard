import {
  assetPath,
  clipHasOwnSound,
  firstSoundAsset,
  timelineWire,
  withProbedFrames,
  withProbedStill,
} from "../../extern/guikit/src/composition-timeline.js";
import { clipDefaultSound, setClipRange } from "../../extern/guikit/src/timeline-editor.js";
import { isChannelSet, soundSource } from "./channel-set.js";

// a channel set cannot be one clip's sound file, so an absorbed clip sound moves to the timeline
export function withChannelSets(timeline, absorbedInto) {
  const grouped = structuredClone(timeline);
  if (grouped.sound !== null && absorbedInto.has(grouped.sound)) grouped.sound = absorbedInto.get(grouped.sound).id;
  for (const clip of grouped.clips) {
    const set = clipHasOwnSound(clip) ? absorbedInto.get(clip.sound.asset) : undefined;
    if (!set) continue;
    clip.sound = clipDefaultSound(clip);
    grouped.sound ??= set.id;
  }
  return grouped;
}

export function clipSoundIsChannelSet(timeline, assetById) {
  return timeline.clips.some((clip) => clipHasOwnSound(clip) && isChannelSet(assetById(clip.sound.asset)));
}

// ffprobe reads a still at its image2 default rate, which the build never takes
export function sourceLengthFields(length) {
  return length.kind === "still" ? { still: true, fps: null } : { frames: length.frames };
}

// clips placed before their source was probed get its length
export function withProbedLength(timeline, asset) {
  return asset.still ? withProbedStill(timeline, asset.id) : withProbedFrames(timeline, asset.id, asset.frames);
}

function frameSpec(frames) {
  return `${frames}f`;
}

// a one-clip timeline's Trim fields are its clip's in and out points, and a still has no trims
export function withClipTrims(timeline, trimStartFrames, trimEndFrames) {
  const [clip, ...laterClips] = timeline.clips;
  if (!clip || laterClips.length > 0 || clip.still || clip.frames === 0) return timeline;
  const outFrame = clip.frames - trimEndFrames;
  if (trimStartFrames >= outFrame) throw new RangeError(`The trims leave none of the clip's ${clip.frames} frames`);
  return setClipRange(timeline, 0, trimStartFrames, outFrame);
}

// a one-clip timeline's Still length field is its still's out point, and empty leaves the hold
export function withStillLength(timeline, holdFrames) {
  const [clip, ...laterClips] = timeline.clips;
  if (!clip?.still || laterClips.length > 0 || holdFrames === 0) return timeline;
  return setClipRange(timeline, 0, 0, holdFrames);
}

export function stillLengthSpec(clip) {
  return frameSpec(clip.outFrame);
}

const NO_TRIM_SPECS = { trimStart: "", trimEnd: "" };

export function clipTrimSpecs(clip) {
  if (clip.still) return NO_TRIM_SPECS;
  const tailFrames = clip.frames - clip.outFrame;
  return { trimStart: clip.inFrame > 0 ? frameSpec(clip.inFrame) : "", trimEnd: tailFrames > 0 ? frameSpec(tailFrames) : "" };
}

// one clip goes as the single-picture request while that request says the same thing
function takesSinglePictureRequest(timeline, clip) {
  if (clip.start !== 0 || clip.layer !== 0) return false;
  // the request plays the source's own audio whenever the source has an audio stream
  if (clip.sound.kind === "none" && !clip.noAudioStream && timeline.sound === null) return false;
  if (timeline.subtitle !== null && clip.subtitle !== null) return false;
  // the single-picture request times its subtitle against the source
  if (timeline.subtitle !== null && clip.inFrame > 0) return false;
  return true;
}

// a still is held for its out point through the request's still length
function singlePictureLengthFields(clip, panel) {
  if (clip.still) return { trimStart: null, trimEnd: null, stillLength: stillLengthSpec(clip) };
  const tailFrames = clip.frames - clip.outFrame;
  return {
    trimStart: clip.inFrame > 0 ? frameSpec(clip.inFrame) : panel.trimStart,
    trimEnd: tailFrames > 0 ? frameSpec(tailFrames) : panel.trimEnd,
  };
}

function singlePictureFields(timeline, clip, assetById, panel) {
  return {
    videoPath: assetPath(assetById, clip.picture),
    ...soundSource(firstSoundAsset({ ...timeline, clips: [clip] }, assetById)),
    subtitle: assetPath(assetById, clip.subtitle ?? timeline.subtitle),
    ...singlePictureLengthFields(clip, panel),
  };
}

// a multi-clip timeline leaves these out, a one-clip timeline sends the panel's own, which submit_job refuses by name
const UNSET_PANEL_FIELDS = { trimStart: null, trimEnd: null, stillLength: null, reelLengthMinutes: 0, splitChapters: false };

// a side the clip trims goes in the wire timeline, and a still's hold is its out point there
function oneClipTrimFields(clip, panel) {
  if (clip.still) return { trimStart: null, trimEnd: null, stillLength: null };
  return {
    trimStart: clip.inFrame > 0 ? null : panel.trimStart,
    trimEnd: clip.outFrame < clip.frames ? null : panel.trimEnd,
  };
}

// a channel set goes as the channel files, so the wire timeline takes its sound per clip
function submittedTimelineWire(timeline, assetById) {
  const sound = timeline.sound === null ? null : assetById(timeline.sound);
  return timelineWire(isChannelSet(sound) ? { ...timeline, sound: null } : timeline, assetById);
}

function timelineFields(timeline, assetById, panel) {
  const sound = timeline.sound === null ? null : assetById(timeline.sound);
  return {
    videoPath: "",
    audioPath: null,
    audioChannelFiles: isChannelSet(sound) ? soundSource(sound).audioChannelFiles : null,
    subtitle: null,
    splitAt: null,
    ...(timeline.clips.length === 1 ? oneClipTrimFields(timeline.clips[0], panel) : UNSET_PANEL_FIELDS),
    timeline: submittedTimelineWire(timeline, assetById),
  };
}

function buildsAsTimeline(timeline) {
  const [firstClip, ...laterClips] = timeline.clips;
  return laterClips.length > 0 || (firstClip !== undefined && !takesSinglePictureRequest(timeline, firstClip));
}

// the timeline submit_job builds, null when the request names one picture or none
export function builtTimelineWire(timeline, assetById) {
  return buildsAsTimeline(timeline) ? submittedTimelineWire(timeline, assetById) : null;
}

const NO_CLIP_SOUND = { kind: "none", asset: null };

// the preview has no player for a channel set
export function previewTimelineWire(timeline, assetById) {
  const sound = timeline.sound === null ? null : assetById(timeline.sound);
  if (!isChannelSet(sound)) return timelineWire(timeline, assetById);
  const silent = { ...timeline, sound: null, clips: timeline.clips.map((clip) => ({ ...clip, sound: NO_CLIP_SOUND })) };
  return timelineWire(silent, assetById);
}

// the submit_job fields the timeline decides, over a request built from the panel
export function timelineRequestFields(timeline, assetById, panel) {
  const [firstClip] = timeline.clips;
  if (!firstClip) {
    return {
      videoPath: "",
      ...soundSource(firstSoundAsset(timeline, assetById)),
      subtitle: assetPath(assetById, timeline.subtitle),
    };
  }
  if (!buildsAsTimeline(timeline)) return singlePictureFields(timeline, firstClip, assetById, panel);
  return timelineFields(timeline, assetById, panel);
}
