import { clipStarts, splitReelAt, totalFrames, withReelStartsInsideTimeline } from "../../extern/guikit/src/timeline-editor.js";
import { isChannelSet, soundSource } from "./channel-set.js";

// postkit's TIMELINE_FORMAT_VERSION, which submit_job checks
const TIMELINE_FORMAT_VERSION = 1;
const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;
const TIMECODE_FIELD_DIGITS = 2;
const SPLIT_TIMECODE_SEPARATOR = ", ";

export const DEFAULT_TIMELINE_FPS = [24, 1];

export const TIMELINE_KIND_BY_ASSET_TYPE = { video: "picture", audio: "sound", subtitle: "subtitle" };

export function emptyTimeline(fps) {
  return { fps, clips: [], sound: null, subtitle: null, reelStarts: [] };
}

function embeddedSound() {
  return { kind: "embedded", asset: null };
}

// 0 frames until the source is probed
function newClip(asset) {
  const frames = asset.frames ?? 0;
  return { picture: asset.id, frames, inFrame: 0, outFrame: frames, sound: embeddedSound(), subtitle: null };
}

function clipHasOwnSound(clip) {
  return clip.sound.kind === "file";
}

function clipHasOwnSubtitle(clip) {
  return clip.subtitle !== null;
}

// a video becomes the last clip, a sound or subtitle covers the whole timeline while nothing else does
export function withImportedAsset(timeline, asset) {
  const placed = structuredClone(timeline);
  if (asset.type === "video") placed.clips.push(newClip(asset));
  if (asset.type === "audio" && placed.sound === null && !placed.clips.some(clipHasOwnSound)) placed.sound = asset.id;
  if (asset.type === "subtitle" && placed.subtitle === null && !placed.clips.some(clipHasOwnSubtitle)) {
    placed.subtitle = asset.id;
  }
  return placed;
}

function placedAssetIds(timeline) {
  const clipAssets = timeline.clips.flatMap((clip) => [clip.picture, clip.sound.asset, clip.subtitle]);
  return new Set([...clipAssets, timeline.sound, timeline.subtitle].filter((id) => id !== null));
}

// an empty timeline takes the first unplaced video, and every unplaced sound or subtitle gets the import rule
export function withUnplacedAssetsFilled(timeline, assets) {
  const placed = placedAssetIds(timeline);
  const unplaced = assets.filter((asset) => !placed.has(asset.id));
  const firstVideo = timeline.clips.length === 0 ? unplaced.find((asset) => asset.type === "video") : undefined;
  const candidates = unplaced.filter((asset) => asset.type !== "video");
  if (firstVideo) candidates.unshift(firstVideo);
  return candidates.reduce(withImportedAsset, timeline);
}

export function withProbedFrames(timeline, assetId, frames) {
  const probed = structuredClone(timeline);
  for (const clip of probed.clips) {
    if (clip.picture !== assetId || clip.frames !== 0) continue;
    clip.frames = frames;
    if (clip.outFrame === 0) clip.outFrame = frames;
  }
  return probed;
}

export function withReelStartingClipsPlaced(composition) {
  const { reelStartingClipIndices, ...placed } = composition;
  // a clip's start is unknown until every clip is probed
  const unprobed = composition.timeline.clips.some((clip) => clip.frames === 0);
  if (!reelStartingClipIndices || unprobed) return composition;
  const starts = clipStarts(composition.timeline);
  placed.timeline = reelStartingClipIndices.map((index) => starts[index]).reduce(splitReelAt, composition.timeline);
  return placed;
}

export function withoutAsset(timeline, assetId) {
  const kept = structuredClone(timeline);
  kept.clips = kept.clips.filter((clip) => clip.picture !== assetId);
  for (const clip of kept.clips) {
    if (clip.sound.asset === assetId) clip.sound = embeddedSound();
    if (clip.subtitle === assetId) clip.subtitle = null;
  }
  if (kept.sound === assetId) kept.sound = null;
  if (kept.subtitle === assetId) kept.subtitle = null;
  return withReelStartsInsideTimeline(kept);
}

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

export function parseFrameRate(rateText) {
  const [numerator, denominator = 1] = String(rateText ?? "").split("/").map(Number);
  if (!(numerator > 0) || !(denominator > 0)) return null;
  return [numerator, denominator];
}

// counted the way postkit's probe counts: the duration at the stream's rate, rounded
export function sourceFrameCount(durationSeconds, rateText) {
  const rate = parseFrameRate(rateText);
  if (!rate || !(durationSeconds > 0)) return 0;
  const [numerator, denominator] = rate;
  return Math.round((durationSeconds * numerator) / denominator);
}

// every clip has to run at the timeline's rate, so the first clip's probed rate sets it
export function withFirstClipRate(timeline, assetById) {
  const firstClip = timeline.clips[0];
  const rate = firstClip ? parseFrameRate(assetById(firstClip.picture)?.fps) : null;
  if (!rate || (rate[0] === timeline.fps[0] && rate[1] === timeline.fps[1])) return timeline;
  return { ...timeline, fps: rate };
}

export function timelineFrameAtSourceFrame(timeline, clipIndex, sourceFrame) {
  return clipStarts(timeline)[clipIndex] + sourceFrame - timeline.clips[clipIndex].inFrame;
}

// the reel starts a split-at field holds, null while it does not parse
export function splitTimecodeFrames(spec, editRate) {
  const timecodes = spec.split(",").map((timecode) => timecode.trim()).filter(Boolean);
  const frames = timecodes.map((timecode) => timecodeFrames(timecode, editRate));
  return frames.includes(null) ? null : frames;
}

function timecodeFrames(timecode, editRate) {
  const fields = timecode.split(":");
  if (fields.length !== 3 && fields.length !== 4) return null;
  if (!fields.every((field) => /^\d+$/.test(field))) return null;
  const [hours, minutes, seconds, frame = 0] = fields.map(Number);
  if (minutes >= SECONDS_PER_MINUTE || seconds >= SECONDS_PER_MINUTE || frame >= editRate) return null;
  return ((hours * MINUTES_PER_HOUR + minutes) * SECONDS_PER_MINUTE + seconds) * editRate + frame;
}

export function withSplitAtFrames(timeline, typedFrames) {
  if (timeline.clips.length > 1 || typedFrames === null) return timeline;
  const split = typedFrames.reduce(splitReelAt, { ...timeline, reelStarts: [] });
  // an unprobed clip has no end to measure against yet
  if ((timeline.clips[0]?.frames ?? 0) === 0) return split;
  return withReelStartsInsideTimeline(split);
}

export function splitTimecodeSpec(frames, editRate) {
  return frames.map((frame) => frameTimecode(frame, editRate)).join(SPLIT_TIMECODE_SEPARATOR);
}

function frameTimecode(frame, editRate) {
  const seconds = Math.floor(frame / editRate);
  const fields = [
    Math.floor(seconds / (SECONDS_PER_MINUTE * MINUTES_PER_HOUR)),
    Math.floor(seconds / SECONDS_PER_MINUTE) % MINUTES_PER_HOUR,
    seconds % SECONDS_PER_MINUTE,
    frame % editRate,
  ];
  return fields.map((field) => String(field).padStart(TIMECODE_FIELD_DIGITS, "0")).join(":");
}

function pathOf(assetById, assetId) {
  return assetId === null ? null : assetById(assetId).path;
}

// what the first clip and the timeline give the sound panel and the ISDCF name
export function firstSoundAsset(timeline, assetById) {
  if (timeline.sound !== null) return assetById(timeline.sound);
  const clip = timeline.clips.find(clipHasOwnSound);
  return clip ? assetById(clip.sound.asset) : null;
}

export function firstSubtitleAsset(timeline, assetById) {
  const subtitle = timeline.subtitle ?? timeline.clips.find(clipHasOwnSubtitle)?.subtitle ?? null;
  return subtitle === null ? null : assetById(subtitle);
}

// one span per reel, each named by the clip it starts in
export function reelSpans(timeline, assetById) {
  if (timeline.clips.length === 0) return [];
  const total = totalFrames(timeline);
  const starts = clipStarts(timeline);
  const bounds = [0, ...timeline.reelStarts.filter((frame) => frame < total), total];
  return bounds.slice(0, -1).map((start, index) => {
    const clipIndex = starts.findLastIndex((clipStart) => clipStart <= start);
    const clip = timeline.clips[clipIndex];
    const sound = firstSoundAsset({ ...timeline, clips: [clip] }, assetById);
    return {
      frames: bounds[index + 1] - start,
      picturePath: pathOf(assetById, clip.picture),
      soundPath: sound?.path ?? sound?.channelFiles?.[0]?.path ?? "",
      subtitlePath: pathOf(assetById, timeline.subtitle ?? clip.subtitle) ?? "",
    };
  });
}

const CLIP_SOUND_WIRE = {
  embedded: () => "Embedded",
  file: (sound, assetById) => ({ File: pathOf(assetById, sound.asset) }),
  none: () => "None",
};

// postkit's Timeline JSON, with paths for asset ids
function timelineWire(timeline, assetById) {
  const [numerator, denominator] = timeline.fps;
  const sound = timeline.sound === null ? null : assetById(timeline.sound);
  return {
    version: TIMELINE_FORMAT_VERSION,
    fps: { numerator, denominator },
    clips: timeline.clips.map((clip) => ({
      picture: pathOf(assetById, clip.picture),
      in_frame: clip.inFrame,
      out_frame: clip.outFrame,
      sound: CLIP_SOUND_WIRE[clip.sound.kind](clip.sound, assetById),
      subtitle: pathOf(assetById, clip.subtitle),
    })),
    sound: sound === null || isChannelSet(sound) ? "PerClip" : { Override: sound.path },
    subtitle: pathOf(assetById, timeline.subtitle),
    reel_starts: [...timeline.reelStarts],
  };
}

function frameSpec(frames) {
  return `${frames}f`;
}

// one clip goes as the single-picture request while that request says the same thing
function takesSinglePictureRequest(timeline, clip, panel) {
  const tailFrames = clip.frames - clip.outFrame;
  if (clip.sound.kind === "none" && timeline.sound === null) return false;
  if (clip.inFrame > 0 && panel.trimStart) return false;
  if (tailFrames > 0 && panel.trimEnd) return false;
  if (timeline.subtitle !== null && clip.subtitle !== null) return false;
  // the single-picture request times its subtitle against the source
  if (timeline.subtitle !== null && clip.inFrame > 0) return false;
  return true;
}

function singlePictureFields(timeline, clip, assetById, panel) {
  const tailFrames = clip.frames - clip.outFrame;
  return {
    videoPath: pathOf(assetById, clip.picture),
    ...soundSource(firstSoundAsset({ ...timeline, clips: [clip] }, assetById)),
    subtitle: pathOf(assetById, clip.subtitle ?? timeline.subtitle),
    trimStart: clip.inFrame > 0 ? frameSpec(clip.inFrame) : panel.trimStart,
    trimEnd: tailFrames > 0 ? frameSpec(tailFrames) : panel.trimEnd,
  };
}

// a multi-clip timeline leaves these out, a one-clip timeline sends the panel's own, which submit_job refuses by name
const UNSET_PANEL_FIELDS = { trimStart: null, trimEnd: null, stillLength: null, reelLengthMinutes: 0, splitChapters: false };

function timelineFields(timeline, assetById) {
  const sound = timeline.sound === null ? null : assetById(timeline.sound);
  return {
    videoPath: "",
    audioPath: null,
    audioChannelFiles: isChannelSet(sound) ? soundSource(sound).audioChannelFiles : null,
    subtitle: null,
    splitAt: null,
    ...(timeline.clips.length === 1 ? {} : UNSET_PANEL_FIELDS),
    timeline: timelineWire(timeline, assetById),
  };
}

// the submit_job fields the timeline decides, over a request built from the panel
export function timelineRequestFields(timeline, assetById, panel) {
  const [firstClip, ...laterClips] = timeline.clips;
  if (!firstClip) {
    return {
      videoPath: "",
      ...soundSource(firstSoundAsset(timeline, assetById)),
      subtitle: pathOf(assetById, timeline.subtitle),
    };
  }
  if (laterClips.length === 0 && takesSinglePictureRequest(timeline, firstClip, panel)) {
    return singlePictureFields(timeline, firstClip, assetById, panel);
  }
  return timelineFields(timeline, assetById);
}
