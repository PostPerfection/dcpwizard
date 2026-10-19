const PICTURE_FILTERS_NEEDED_HINT =
  "The container view needs a video file source. A JPEG 2000 source plays through the grok player, which has no picture filters.";
const UPPER_LAYER_HINT =
  "The container view is off while a clip sits above layer 0: the preview renders the stretches under it onto the container itself.";

// the hint saying why the container view is off, null while it can be turned on
export function containerViewRefusal(timeline, previewedClipTakesFilters) {
  if (timeline.clips.some((clip) => clip.layer > 0)) return UPPER_LAYER_HINT;
  if (!previewedClipTakesFilters) return PICTURE_FILTERS_NEEDED_HINT;
  return null;
}
