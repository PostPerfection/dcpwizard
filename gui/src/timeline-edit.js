import { withFirstClipRate } from "../../extern/guikit/src/composition-timeline.js";
import { cancelTimelineRender } from "../../extern/guikit/src/preview.js";
import { clipSoundIsChannelSet } from "./composition-timeline.js";

// the timeline an edit stores, null when the edit is refused
export function acceptTimelineEdit(timeline, assetById) {
  if (clipSoundIsChannelSet(timeline, assetById)) return null;
  // a render in flight is of the timeline before the edit
  cancelTimelineRender();
  return withFirstClipRate(timeline, assetById);
}
