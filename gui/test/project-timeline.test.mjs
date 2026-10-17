import assert from "node:assert/strict";
import test from "node:test";

import { projectTimelineEntries, segmentSpan } from "../src/project-timeline.js";

const SPANS = [
  { frames: 240, picture: { path: "/media/reel1.mov" }, sound: { path: "/media/reel1.wav" }, subtitle: { path: "/media/reel1.xml" } },
  { frames: 120, picture: { path: "/media/reel2.mov" }, sound: null, subtitle: null },
  {
    frames: 48,
    picture: { path: "/media/reel3.mov" },
    sound: { path: null, channelFiles: [{ path: "/media/mix_L.wav", lane: "L" }, { path: "/media/mix_R.wav", lane: "R" }] },
    subtitle: null,
  },
];

test("each reel span becomes a timeline entry with its files and length", () => {
  const [first, second, third] = projectTimelineEntries(SPANS, 24);

  assert.equal(first.reel_number, 1);
  assert.equal(first.duration_frames, 240);
  assert.equal(first.edit_rate, "24 1");
  assert.equal(first.picture_file, "/media/reel1.mov");
  assert.equal(first.subtitle_file, "/media/reel1.xml");
  assert.equal(second.reel_number, 2);
  assert.equal(second.sound_file, "");
  assert.equal(second.subtitle_file, "");
  assert.equal(third.sound_file, "/media/mix_L.wav", "a channel set shows its first file");
});

test("a reel spans its share of the composition", () => {
  const reel = { reel_number: 2, startFrame: 240, duration_frames: 120 };

  assert.deepEqual(segmentSpan(reel, 3, 480), { leftPercent: 50, widthPercent: 25 });
});

test("with no reel length known each reel gets an equal share", () => {
  const reel = { reel_number: 2, startFrame: 0, duration_frames: 0 };

  assert.deepEqual(segmentSpan(reel, 4, 0), { leftPercent: 25, widthPercent: 25 });
});
