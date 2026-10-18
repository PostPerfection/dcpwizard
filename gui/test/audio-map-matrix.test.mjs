import assert from "node:assert/strict";
import test from "node:test";

import { audioMapHint, audioMapTable } from "../src/audio-map-matrix.js";

const LANES = ["L", "R", "C", "LFE", "Ls", "Rs", "Lc", "Rc"];
const UPMIX_REFUSAL = "The audio mapping and the stereo upmix both decide which DCP lane each channel lands on: use one or the other";
const UPMIX_IN_CHARGE = "Typed gains are off: the stereo upmix places the channels itself.";

function panel(fields) {
  return {
    lanes: LANES,
    channelNames: ["Channel 1", "Channel 2"],
    autoRoutes: ["L", "R"],
    upmixLanes: [],
    upmixDescription: null,
    typedGainsOff: null,
    typedMapRefusal: null,
    ...fields,
  };
}

test("the upmix lanes are marked in the header and typed cells are disabled", () => {
  const upmixed = panel({
    autoRoutes: [null, null],
    upmixLanes: ["L", "R", "C", "LFE", "Ls", "Rs"],
    typedGainsOff: UPMIX_IN_CHARGE,
    typedMapRefusal: UPMIX_REFUSAL,
  });
  const table = audioMapTable(upmixed);
  assert.equal((table.match(/class="upmix-lane"/g) ?? []).length, 6);
  assert.match(table, /<th>Lc<\/th>/);
  assert.equal((table.match(/ disabled /g) ?? []).length, 2 * LANES.length);
  assert.doesNotMatch(table, /auto-routed/);
});

test("an untouched matrix has editable cells on the dotted auto routes", () => {
  const table = audioMapTable(panel({}));
  assert.doesNotMatch(table, /disabled|upmix-lane/);
  assert.match(table, /class="auto-routed" placeholder="auto" data-input="2" data-lane="R"/);
});

test("the hint names the upmix, the dotted cells and who places the channels", () => {
  const upmixed = panel({
    autoRoutes: [null, null],
    upmixDescription: "Variant A makes L and R.",
    typedGainsOff: UPMIX_IN_CHARGE,
    typedMapRefusal: UPMIX_REFUSAL,
  });
  assert.equal(audioMapHint(upmixed, { hasTypedGains: false }), `Variant A makes L and R. ${UPMIX_IN_CHARGE}`);
  assert.match(audioMapHint(panel({}), { hasTypedGains: false }), /^Dotted cells show where each channel goes\. Click a cell/);
});

test("the refusal shows only when a typed gain sits beside the upmix", () => {
  const upmixed = panel({ autoRoutes: [null, null], typedGainsOff: UPMIX_IN_CHARGE, typedMapRefusal: UPMIX_REFUSAL });
  assert.equal(audioMapHint(upmixed, { hasTypedGains: true }), `Typed gains are off. ${UPMIX_REFUSAL}.`);
  assert.doesNotMatch(audioMapHint(panel({}), { hasTypedGains: true }), /Typed gains are off/);
});
