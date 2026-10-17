import { soundSource } from "./channel-set.js";

const FULL_TRACK_PERCENT = 100;

// a channel set shows its first file
function soundFilePath(sound) {
  const { audioPath, audioChannelFiles } = soundSource(sound);
  return audioPath ?? audioChannelFiles?.[0] ?? "";
}

// shaped like get_timeline's entries, one per reel span of the composition's timeline
export function projectTimelineEntries(spans, editRate) {
  return spans.map((span, index) => ({
    reel_id: String(index + 1),
    reel_number: index + 1,
    duration_frames: span.frames,
    entry_point: 0,
    edit_rate: `${editRate} 1`,
    picture_asset_id: "",
    sound_asset_id: "",
    subtitle_asset_id: "",
    picture_file: span.picture.path,
    sound_file: soundFilePath(span.sound),
    subtitle_file: span.subtitle?.path ?? "",
  }));
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
