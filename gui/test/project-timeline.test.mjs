import assert from "node:assert/strict";
import test from "node:test";

import { projectTimelineEntries, segmentSpan } from "../src/project-timeline.js";

const SPANS = [
  { frames: 240, picturePath: "/media/reel1.mov", soundPath: "/media/reel1.wav", subtitlePath: "/media/reel1.xml" },
  { frames: 120, picturePath: "/media/reel2.mov", soundPath: "", subtitlePath: "" },
];

test("each reel span becomes a timeline entry with its files and length", () => {
  const [first, second] = projectTimelineEntries(SPANS, 24);

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
