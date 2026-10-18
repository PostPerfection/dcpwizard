import {
  assetPath,
  clipHasOwnSound,
  embeddedSound,
  firstSoundAsset,
  timelineWire,
} from "../../extern/guikit/src/composition-timeline.js";
import { setClipRange } from "../../extern/guikit/src/timeline-editor.js";
import { isChannelSet, soundSource } from "./channel-set.js";

// a channel set cannot be one clip's sound file, so an absorbed clip sound moves to the timeline
export function withChannelSets(timeline, absorbedInto) {
  const grouped = structuredClone(timeline);
  if (grouped.sound !== null && absorbedInto.has(grouped.sound)) grouped.sound = absorbedInto.get(grouped.sound).id;
  for (const clip of grouped.clips) {
    const set = clipHasOwnSound(clip) ? absorbedInto.get(clip.sound.asset) : undefined;
    if (!set) continue;
    clip.sound = embeddedSound();
    grouped.sound ??= set.id;
  }
  return grouped;
}

export function clipSoundIsChannelSet(timeline, assetById) {
  return timeline.clips.some((clip) => clipHasOwnSound(clip) && isChannelSet(assetById(clip.sound.asset)));
}

function frameSpec(frames) {
  return `${frames}f`;
}

// a one-clip timeline's Trim fields are its clip's in and out points
export function withClipTrims(timeline, trimStartFrames, trimEndFrames) {
  const [clip, ...laterClips] = timeline.clips;
  if (!clip || laterClips.length > 0 || clip.frames === 0) return timeline;
  const outFrame = clip.frames - trimEndFrames;
  if (trimStartFrames >= outFrame) throw new RangeError(`The trims leave none of the clip's ${clip.frames} frames`);
  return setClipRange(timeline, 0, trimStartFrames, outFrame);
}

export function clipTrimSpecs(clip) {
  const tailFrames = clip.frames - clip.outFrame;
  return { trimStart: clip.inFrame > 0 ? frameSpec(clip.inFrame) : "", trimEnd: tailFrames > 0 ? frameSpec(tailFrames) : "" };
}

// one clip goes as the single-picture request while that request says the same thing
function takesSinglePictureRequest(timeline, clip) {
  if (clip.start !== 0 || clip.layer !== 0) return false;
  if (clip.sound.kind === "none" && timeline.sound === null) return false;
  if (timeline.subtitle !== null && clip.subtitle !== null) return false;
  // the single-picture request times its subtitle against the source
  if (timeline.subtitle !== null && clip.inFrame > 0) return false;
  return true;
}

function singlePictureFields(timeline, clip, assetById, panel) {
  const tailFrames = clip.frames - clip.outFrame;
  return {
    videoPath: assetPath(assetById, clip.picture),
    ...soundSource(firstSoundAsset({ ...timeline, clips: [clip] }, assetById)),
    subtitle: assetPath(assetById, clip.subtitle ?? timeline.subtitle),
    trimStart: clip.inFrame > 0 ? frameSpec(clip.inFrame) : panel.trimStart,
    trimEnd: tailFrames > 0 ? frameSpec(tailFrames) : panel.trimEnd,
  };
}

// a multi-clip timeline leaves these out, a one-clip timeline sends the panel's own, which submit_job refuses by name
const UNSET_PANEL_FIELDS = { trimStart: null, trimEnd: null, stillLength: null, reelLengthMinutes: 0, splitChapters: false };

// a side the clip trims goes in the wire timeline
function oneClipTrimFields(clip, panel) {
  return {
    trimStart: clip.inFrame > 0 ? null : panel.trimStart,
    trimEnd: clip.outFrame < clip.frames ? null : panel.trimEnd,
  };
}

// a channel set goes as the channel files, so the wire timeline takes its sound per clip
function timelineFields(timeline, assetById, panel) {
  const sound = timeline.sound === null ? null : assetById(timeline.sound);
  const soundIsChannelSet = isChannelSet(sound);
  return {
    videoPath: "",
    audioPath: null,
    audioChannelFiles: soundIsChannelSet ? soundSource(sound).audioChannelFiles : null,
    subtitle: null,
    splitAt: null,
    ...(timeline.clips.length === 1 ? oneClipTrimFields(timeline.clips[0], panel) : UNSET_PANEL_FIELDS),
    timeline: timelineWire(soundIsChannelSet ? { ...timeline, sound: null } : timeline, assetById),
  };
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
  const [firstClip, ...laterClips] = timeline.clips;
  if (!firstClip) {
    return {
      videoPath: "",
      ...soundSource(firstSoundAsset(timeline, assetById)),
      subtitle: assetPath(assetById, timeline.subtitle),
    };
  }
  if (laterClips.length === 0 && takesSinglePictureRequest(timeline, firstClip)) {
    return singlePictureFields(timeline, firstClip, assetById, panel);
  }
  return timelineFields(timeline, assetById, panel);
}
