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
    sound_file: reel.sound?.path || "",
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
