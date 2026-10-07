import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { COLOUR_NOT_DECLARED, detectedValues, probeMayFill } from "../src/detected-values.js";

const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");

function selectOptionValues(id) {
  const select = new RegExp(`<select id="${id}">([\\s\\S]*?)</select>`).exec(html);
  return [...select[1].matchAll(/<option value="([^"]*)"/g)].map((match) => match[1]);
}

const FRAMERATE_OPTIONS = selectOptionValues("prop-framerate");
const SOURCE_COLOURSPACE_OPTIONS = selectOptionValues("prop-source-colourspace");

const HD_PRORES = { width: 1920, height: 1080, fps: "24000/1001", colorSpace: "bt709", bitRate: 179_000_000 };

function detected(probe) {
  return detectedValues({ width: 1920, height: 1080, fps: "24/1", ...probe }, FRAMERATE_OPTIONS);
}

test("a clip tagged with only the bt709 matrix is Rec. 709", () => {
  assert.equal(detected({ colorSpace: "bt709" }).sourceColourspace, "rec709");
});

test("bt2020 primaries are Rec. 2020", () => {
  assert.equal(detected({ colorPrimaries: "bt2020", colorTransfer: "smpte2084", colorSpace: "bt2020nc" }).sourceColourspace, "rec2020");
});

test("DCI P3 primaries are the p3 option and P3-D65 primaries match none", () => {
  assert.equal(detected({ colorPrimaries: "smpte431" }).sourceColourspace, "p3");
  assert.equal(detected({ colorPrimaries: "smpte432", colorSpace: "bt709" }).sourceColourspace, null);
});

test("SMPTE 428 primaries or transfer are X'Y'Z'", () => {
  assert.equal(detected({ colorPrimaries: "smpte428" }).sourceColourspace, "xyz");
  assert.equal(detected({ colorTransfer: "smpte428" }).sourceColourspace, "xyz");
});

test("every colour space the probe picks is an option of the select", () => {
  for (const tag of ["bt709", "bt2020", "smpte431", "smpte428"]) {
    assert.ok(SOURCE_COLOURSPACE_OPTIONS.includes(detected({ colorPrimaries: tag }).sourceColourspace), tag);
  }
});

test("a clip with no colour tags leaves the colour space alone and says so", () => {
  const values = detected({ colorPrimaries: "unknown" });
  assert.equal(values.sourceColourspace, null);
  assert.ok(values.hint.includes(COLOUR_NOT_DECLARED), values.hint);
});

test("a whole frame rate picks its option and 23.976 picks none", () => {
  assert.equal(detected({ fps: "25/1" }).framerate, "25");
  assert.equal(detected({ fps: "24000/1001" }).framerate, null);
});

test("the hint lists size, rate, colour and bit rate", () => {
  assert.equal(
    detectedValues(HD_PRORES, FRAMERATE_OPTIONS).hint,
    "From the source: 1920x1080, 23.976 fps, bt709, 179 Mbit/s. Edit any field to override.",
  );
});

test("the hint leaves out a bit rate the source does not carry", () => {
  assert.equal(
    detected({ colorSpace: "bt709", bitRate: null }).hint,
    "From the source: 1920x1080, 24 fps, bt709. Edit any field to override.",
  );
});

test("a probe fills a field at its default or one it filled before", () => {
  const untouched = { detected: false, profileDriven: false, edited: false, atDefault: true };
  assert.equal(probeMayFill(untouched), true);
  assert.equal(probeMayFill({ ...untouched, atDefault: false, detected: true }), true);
});

test("a probe never fills an edited, profile-set or restored field", () => {
  const untouched = { detected: false, profileDriven: false, edited: false, atDefault: true };
  assert.equal(probeMayFill({ ...untouched, edited: true }), false);
  assert.equal(probeMayFill({ ...untouched, profileDriven: true }), false);
  assert.equal(probeMayFill({ ...untouched, atDefault: false }), false);
});
