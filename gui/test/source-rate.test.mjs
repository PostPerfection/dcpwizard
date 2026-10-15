import assert from "node:assert/strict";
import test from "node:test";

import { editRateOf } from "../src/source-rate.js";

const DEFAULT_RATE = 24;

test("Auto packages at the rate the source maps to", () => {
  assert.equal(editRateOf("auto", "25/1", DEFAULT_RATE), 25);
  assert.equal(editRateOf("auto", "30000/1001", DEFAULT_RATE), 30);
  assert.equal(editRateOf("auto", "60/1", DEFAULT_RATE), 60);
  assert.equal(editRateOf("auto", "24000/1001", DEFAULT_RATE), 24);
});

test("Auto with no probed source falls back to the default rate", () => {
  assert.equal(editRateOf("auto", undefined, DEFAULT_RATE), DEFAULT_RATE);
});

test("a chosen rate is kept whatever the source runs at", () => {
  assert.equal(editRateOf("48", "30000/1001", DEFAULT_RATE), 48);
});
