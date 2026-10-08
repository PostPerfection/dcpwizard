const FULL_TRACK_PERCENT = 100;

// shaped like get_timeline's entries
export function projectTimelineEntries(reels, durationsFrames, editRate) {
  return reels.map((reel, index) => ({
    reel_id: String(reel.id),
    reel_number: index + 1,
    duration_frames: durationsFrames[index] || 0,
    entry_point: 0,
    edit_rate: `${editRate} 1`,
    picture_asset_id: "",
    sound_asset_id: "",
    subtitle_asset_id: "",
    picture_file: reel.picture?.path || "",
    sound_file: reel.sound?.path || reel.sound?.channelFiles?.[0]?.path || "",
    subtitle_file: reel.subtitle?.path || "",
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

// the reel a timeline frame falls in, and how far into that reel it is
export function reelPositionAtFrame(reels, frame) {
  const index = reels.findIndex((reel) => frame < reel.startFrame + reel.duration_frames);
  const reelIndex = index === -1 ? reels.length - 1 : index;
  const reel = reels[reelIndex];
  return { reelIndex, seconds: Math.max(0, frame - reel.startFrame) / reel.fps };
}

// the timeline frame a preview position stands for while the preview plays one reel's picture
export function timelineFrameOfReelPosition(reel, seconds) {
  const intoReel = Math.min(Math.floor(seconds * reel.fps), reel.duration_frames);
  return reel.startFrame + intoReel;
}
