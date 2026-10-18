import { reelSpans } from "../../extern/guikit/src/composition-timeline.js";
import { baseClipsInOrder, clipEnd, totalFrames } from "../../extern/guikit/src/timeline-editor.js";
import { soundSource } from "./channel-set.js";

const FULL_TRACK_PERCENT = 100;
const CLIP_NAME_SEPARATOR = ", ";
const EMBEDDED_SOUND_LABEL = "embedded";
const GAP_PICTURE_LABEL = "black";

// a channel set shows its first file
function soundFilePath(sound) {
  const { audioPath, audioChannelFiles } = soundSource(sound);
  return audioPath ?? audioChannelFiles?.[0] ?? "";
}

// a layer 0 clip is in every reel from the one holding its first frame to the one holding its last
export function reelClipIndices(timeline) {
  const total = totalFrames(timeline);
  const reelStartFrames = [0, ...timeline.reelStarts.filter((frame) => frame < total)];
  const reelAt = (frame) => reelStartFrames.findLastIndex((start) => start <= frame);
  const reels = reelStartFrames.map(() => []);
  for (const { index, clip } of baseClipsInOrder(timeline)) {
    const lastFrame = Math.max(clip.start, clipEnd(clip) - 1);
    for (let reel = reelAt(clip.start); reel <= reelAt(lastFrame); reel++) reels[reel].push(index);
  }
  return reels;
}

function clipSoundLabel(clip, assetById) {
  if (clip.sound.kind === "embedded") return EMBEDDED_SOUND_LABEL;
  if (clip.sound.kind === "file") return assetById(clip.sound.asset).name;
  return null;
}

function reelSoundLabel(timeline, clips, assetById) {
  if (timeline.sound !== null) return assetById(timeline.sound).name;
  const labels = clips.map((clip) => clipSoundLabel(clip, assetById)).filter((label) => label !== null);
  return [...new Set(labels)].join(CLIP_NAME_SEPARATOR);
}

function reelPictureNames(span, clips, assetById) {
  const clipNames = clips.map((clip) => assetById(clip.picture).name);
  const names = span.picture === null ? [GAP_PICTURE_LABEL, ...clipNames] : clipNames;
  return names.join(CLIP_NAME_SEPARATOR);
}

// shaped like get_timeline's entries, one per reel span of the composition's timeline, plus the names the reel covers
export function projectTimelineEntries(timeline, assetById, editRate) {
  const clipIndicesByReel = reelClipIndices(timeline);
  return reelSpans(timeline, assetById).map((span, index) => {
    const clips = clipIndicesByReel[index].map((clipIndex) => timeline.clips[clipIndex]);
    return {
      reel_id: String(index + 1),
      reel_number: index + 1,
      duration_frames: span.frames,
      entry_point: 0,
      edit_rate: `${editRate} 1`,
      picture_asset_id: "",
      sound_asset_id: "",
      subtitle_asset_id: "",
      picture_file: span.picture?.path ?? "",
      sound_file: soundFilePath(span.sound),
      subtitle_file: span.subtitle?.path ?? "",
      picture_names: reelPictureNames(span, clips, assetById),
      sound_names: reelSoundLabel(timeline, clips, assetById),
    };
  });
}

// with no reel length known yet each reel gets an equal share of the track
export function segmentSpan(reel, reelCount, totalFrames) {
  if (totalFrames === 0) {
    const widthPercent = FULL_TRACK_PERCENT / reelCount;
    return { leftPercent: (reel.reel_number - 1) * widthPercent, widthPercent };
  }
  return {
    leftPercent: (reel.startFrame / totalFrames) * FULL_TRACK_PERCENT,
    widthPercent: (reel.duration_frames / totalFrames) * FULL_TRACK_PERCENT,
  };
}
