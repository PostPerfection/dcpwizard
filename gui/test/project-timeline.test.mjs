import assert from "node:assert/strict";
import test from "node:test";

import { emptyTimeline, withImportedAsset } from "../../extern/guikit/src/composition-timeline.js";
import { splitReelAt } from "../../extern/guikit/src/timeline-editor.js";
import { projectTimelineEntries, reelClipIndices, segmentSpan } from "../src/project-timeline.js";

const ASSETS = [
  { id: 1, type: "video", path: "/media/movie.mov", name: "movie.mov", frames: 240 },
  { id: 2, type: "video", path: "/media/trailer.mov", name: "trailer.mov", frames: 120 },
  { id: 3, type: "subtitle", path: "/media/movie.xml", name: "movie.xml" },
  {
    id: 4,
    type: "audio",
    path: null,
    name: "mix",
    channelFiles: [{ path: "/media/mix_L.wav", lane: "L" }, { path: "/media/mix_R.wav", lane: "R" }],
  },
];

function assetById(assetId) {
  return ASSETS.find((asset) => asset.id === assetId);
}

function timelineOf(...assetIds) {
  return assetIds.map(assetById).reduce(withImportedAsset, emptyTimeline([24, 1]));
}

test("each reel span becomes a timeline entry with its files and length", () => {
  const timeline = splitReelAt(timelineOf(1, 2), 240);
  timeline.clips[0].subtitle = 3;
  timeline.clips[1].sound = { kind: "file", asset: 4 };

  const [first, second] = projectTimelineEntries(timeline, assetById, 24);

  assert.equal(first.reel_number, 1);
  assert.equal(first.duration_frames, 240);
  assert.equal(first.edit_rate, "24 1");
  assert.equal(first.picture_file, "/media/movie.mov");
  assert.equal(first.subtitle_file, "/media/movie.xml");
  assert.equal(first.sound_file, "");
  assert.equal(second.reel_number, 2);
  assert.equal(second.subtitle_file, "");
  assert.equal(second.sound_file, "/media/mix_L.wav", "a channel set shows its first file");
  assert.equal(second.sound_names, "mix");
});

test("a reel with no reel start names every clip it covers in order", () => {
  const entries = projectTimelineEntries(timelineOf(1, 2), assetById, 24);

  assert.equal(entries.length, 1);
  assert.equal(entries[0].picture_names, "movie.mov, trailer.mov");
});

test("a clip running over a reel start is in both reels", () => {
  assert.deepEqual(reelClipIndices(splitReelAt(timelineOf(1, 2), 120)), [[0], [0, 1]]);
});

test("a clip with embedded sound gives its reel a sound segment", () => {
  const [entry] = projectTimelineEntries(timelineOf(1), assetById, 24);

  assert.equal(entry.sound_file, "");
  assert.equal(entry.sound_names, "embedded");
});

test("a reel whose clips have no sound has no sound segment", () => {
  const timeline = timelineOf(1);
  timeline.clips[0].sound = { kind: "none", asset: null };

  assert.equal(projectTimelineEntries(timeline, assetById, 24)[0].sound_names, "");
});

test("a reel spans its share of the composition", () => {
  const reel = { reel_number: 2, startFrame: 240, duration_frames: 120 };

  assert.deepEqual(segmentSpan(reel, 3, 480), { leftPercent: 50, widthPercent: 25 });
});

test("with no reel length known each reel gets an equal share", () => {
  const reel = { reel_number: 2, startFrame: 0, duration_frames: 0 };

  assert.deepEqual(segmentSpan(reel, 4, 0), { leftPercent: 25, widthPercent: 25 });
});
