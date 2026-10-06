import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { PREFERRED_PROJECT_CONTROLS, projectDefaultsFromPreferences } from "../src/project-defaults.js";

const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");

function selectOptionValues(id) {
  const select = new RegExp(`<select id="${id}">([\\s\\S]*?)</select>`).exec(html);
  if (!select) return undefined;
  return [...select[1].matchAll(/<option value="([^"]*)"/g)].map((match) => match[1]);
}

const PROPERTIES_OPTIONS = Object.fromEntries(
  PREFERRED_PROJECT_CONTROLS
    .map(([key, id]) => [key, selectOptionValues(id)])
    .filter(([, values]) => values),
);

const SAVED = { standard: "Interop", resolution: "2k-flat", framerate: 25, bandwidth: 180 };

test("the four Properties controls a new project takes from Settings", () => {
  assert.deepEqual(PREFERRED_PROJECT_CONTROLS, [
    ["standard", "prop-standard"],
    ["resolution", "prop-resolution"],
    ["framerate", "prop-framerate"],
    ["bandwidth", "prop-bandwidth"],
  ]);
});

test("the saved standard maps onto the Properties option spelled in lowercase", () => {
  assert.equal(projectDefaultsFromPreferences(SAVED, PROPERTIES_OPTIONS).standard, "interop");
});

test("the saved resolution is a Properties container", () => {
  assert.equal(projectDefaultsFromPreferences(SAVED, PROPERTIES_OPTIONS).resolution, "2k-flat");
});

test("the saved frame rate and bandwidth become the form's string values", () => {
  const defaults = projectDefaultsFromPreferences(SAVED, PROPERTIES_OPTIONS);

  assert.equal(defaults.framerate, "25");
  assert.equal(defaults.bandwidth, "180");
});

test("a saved value Properties does not offer is left out", () => {
  const defaults = projectDefaultsFromPreferences({ ...SAVED, framerate: 120, resolution: "8k" }, PROPERTIES_OPTIONS);

  assert.equal("framerate" in defaults, false);
  assert.equal("resolution" in defaults, false);
});
