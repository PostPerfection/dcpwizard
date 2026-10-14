import assert from "node:assert/strict";
import test from "node:test";

import { boundedPropertiesWidth, initProjectPropertiesLayout } from "../src/project-properties-layout.js";

const POINTER = 7;
const PRIMARY_BUTTON = 0;
const PRIMARY_BUTTON_HELD = 1;
const WORKSPACE_WIDTH = 1000;
const START_WIDTH = 360;

function fakeClassList() {
  const values = new Set();
  return {
    contains: (value) => values.has(value),
    toggle(value, enabled) {
      if (enabled) values.add(value);
      else values.delete(value);
    },
  };
}

function fakeControl() {
  return {
    handlers: {},
    attributes: {},
    addEventListener(name, handler) {
      this.handlers[name] = handler;
    },
    setAttribute(name, value) {
      this.attributes[name] = value;
    },
  };
}

function setUp(savedValues = {}) {
  const saved = new Map(Object.entries(savedValues));
  const workspace = {
    classList: fakeClassList(),
    width: WORKSPACE_WIDTH,
    style: {
      setProperty(_, value) {
        panel.width = Number.parseInt(value);
      },
    },
    getBoundingClientRect: () => ({ width: workspace.width }),
  };
  const panel = { width: START_WIDTH, getBoundingClientRect: () => ({ width: panel.width }) };
  const handle = {
    ...fakeControl(),
    hidden: true,
    captured: new Set(),
    setPointerCapture(pointer) {
      this.captured.add(pointer);
    },
    hasPointerCapture(pointer) {
      return this.captured.has(pointer);
    },
    releasePointerCapture(pointer) {
      this.captured.delete(pointer);
    },
  };
  const position = { ...fakeControl(), value: "below" };
  globalThis.document = {
    querySelector: () => workspace,
    getElementById(id) {
      return { properties: panel, "properties-resize-handle": handle, "properties-position": position }[id];
    },
  };
  globalThis.localStorage = {
    getItem: (key) => saved.get(key) ?? null,
    setItem: (key, value) => saved.set(key, value),
  };
  initProjectPropertiesLayout("wizard");
  const fireHandle = (name, event = {}) => {
    const fired = {
      pointerId: POINTER,
      defaultPrevented: false,
      preventDefault: () => (fired.defaultPrevented = true),
      ...event,
    };
    handle.handlers[name](fired);
    return fired;
  };
  return { workspace, panel, handle, position, saved, fireHandle };
}

test("properties start below the assets", () => {
  const { workspace, handle, position } = setUp();

  assert.equal(workspace.classList.contains("properties-on-right"), false);
  assert.equal(handle.hidden, true);
  assert.equal(position.value, "below");
});

test("the saved position puts properties on the right", () => {
  const { workspace, handle, position } = setUp({ "wizard-properties-position": "right" });

  assert.equal(workspace.classList.contains("properties-on-right"), true);
  assert.equal(handle.hidden, false);
  assert.equal(position.value, "right");
});

test("changing the position saves it", () => {
  const { workspace, handle, position, saved } = setUp();
  position.value = "right";
  position.handlers.change();

  assert.equal(workspace.classList.contains("properties-on-right"), true);
  assert.equal(handle.hidden, false);
  assert.equal(saved.get("wizard-properties-position"), "right");
});

test("dragging left widens the right panel and saves its width", () => {
  const { panel, handle, saved, fireHandle } = setUp({ "wizard-properties-position": "right" });
  fireHandle("pointerdown", { button: PRIMARY_BUTTON, clientX: 700 });
  fireHandle("pointermove", { buttons: PRIMARY_BUTTON_HELD, clientX: 620 });

  assert.equal(panel.width, START_WIDTH + 80);
  fireHandle("pointerup");
  assert.equal(saved.get("wizard-properties-width"), String(START_WIDTH + 80));
  assert.equal(handle.hasPointerCapture(POINTER), false);
});

test("the panel width stays between its minimum and sixty percent", () => {
  assert.equal(boundedPropertiesWidth(100, WORKSPACE_WIDTH), 280);
  assert.equal(boundedPropertiesWidth(900, WORKSPACE_WIDTH), 600);
});

test("arrow keys resize the panel", () => {
  const { panel, saved, fireHandle } = setUp({ "wizard-properties-position": "right" });

  assert.equal(fireHandle("keydown", { key: "ArrowLeft" }).defaultPrevented, true);
  assert.equal(panel.width, START_WIDTH + 24);
  assert.equal(saved.get("wizard-properties-width"), String(START_WIDTH + 24));
});
