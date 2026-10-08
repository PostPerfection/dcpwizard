import assert from "node:assert/strict";
import test from "node:test";

import {
  projectTimelineEntries,
  reelPositionAtFrame,
  segmentSpan,
  timelineFrameOfReelPosition,
} from "../src/project-timeline.js";

const REELS = [
  { id: 4, picture: { path: "/media/reel1.mov" }, sound: { path: "/media/reel1.wav" }, subtitle: { path: "/media/reel1.xml" } },
  { id: 9, picture: { path: "/media/reel2.mov" }, sound: null, subtitle: null },
];

test("each project reel becomes a timeline entry with its files and length", () => {
  const [first, second] = projectTimelineEntries(REELS, [240, 120], 24);

  assert.equal(first.reel_number, 1);
  assert.equal(first.duration_frames, 240);
  assert.equal(first.edit_rate, "24 1");
  assert.equal(first.picture_file, "/media/reel1.mov");
  assert.equal(first.subtitle_file, "/media/reel1.xml");
  assert.equal(second.reel_number, 2);
  assert.equal(second.sound_file, "");
  assert.equal(second.subtitle_file, "");
});

test("a reel spans its share of the composition", () => {
  const reel = { reel_number: 2, startFrame: 240, duration_frames: 120 };

  assert.deepEqual(segmentSpan(reel, 3, 480), { leftPercent: 50, widthPercent: 25 });
});

test("with no reel length known each reel gets an equal share", () => {
  const reel = { reel_number: 2, startFrame: 0, duration_frames: 0 };

  assert.deepEqual(segmentSpan(reel, 4, 0), { leftPercent: 25, widthPercent: 25 });
});

const PLACED_REELS = [
  { startFrame: 0, duration_frames: 240, fps: 24 },
  { startFrame: 240, duration_frames: 120, fps: 24 },
];

test("a frame in the second reel is that many seconds into the second reel", () => {
  assert.deepEqual(reelPositionAtFrame(PLACED_REELS, 300), { reelIndex: 1, seconds: 2.5 });
  assert.deepEqual(reelPositionAtFrame(PLACED_REELS, 0), { reelIndex: 0, seconds: 0 });
});

test("a frame past the end lands in the last reel", () => {
  assert.equal(reelPositionAtFrame(PLACED_REELS, 999).reelIndex, 1);
});

test("a preview position in the second reel's picture is offset by the reels before it", () => {
  assert.equal(timelineFrameOfReelPosition(PLACED_REELS[1], 2.5), 300);
  // a position past the reel's end stays inside the reel
  assert.equal(timelineFrameOfReelPosition(PLACED_REELS[1], 60), 360);
});
