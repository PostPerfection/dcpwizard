import assert from 'node:assert/strict';
import test from 'node:test';

import { progressDisplay, formatTime } from '../src/progress-format.js';

const ENCODE = {
  stage: 'encode',
  message: 'Frame 1200/6000',
  frame: 1200,
  total_frames: 6000,
  fps: 24,
  elapsed_secs: 50,
  percent: 20,
};

const PACKAGE = {
  stage: 'package',
  message: 'wrapping sound',
  frame: 0,
  total_frames: 0,
  fps: 0,
  elapsed_secs: 630,
  percent: 55,
};

test('an encode with a rate and frames left gets an ETA from the rate', () => {
  const display = progressDisplay(ENCODE);

  assert.equal(display.stats, '50s 24.0fps ETA 3m20s');
  assert.equal(display.percent, 20);
});

test('the stage line carries the stage name and the message', () => {
  assert.equal(progressDisplay(ENCODE).stage, 'Encode: Frame 1200/6000');
});

test('a package stage with a percent but no rate shows no ETA', () => {
  const display = progressDisplay(PACKAGE);

  assert.equal(display.stats, '10m30s');
  assert.equal(display.percent, 55);
  assert.equal(display.stage, 'Package: wrapping sound');
});

test('a picture wrap reports its own rate and the frames it has left', () => {
  const display = progressDisplay({
    ...PACKAGE,
    message: 'wrapping the picture',
    frame: 3000,
    total_frames: 9000,
    fps: 120,
    percent: 28.3,
  });

  assert.equal(display.stats, '10m30s 120.0fps ETA 50s');
});

test('a stage with no measurable progress leaves the bar indeterminate', () => {
  const display = progressDisplay({ ...PACKAGE, message: 'removing intermediate frames', percent: null });

  assert.equal(display.percent, null);
  assert.equal(display.stage, 'Package: removing intermediate frames');
  assert.equal(display.stats, '10m30s');
});

test('the last frame of a wrap has nothing left to wait for', () => {
  const display = progressDisplay({ ...PACKAGE, frame: 9000, total_frames: 9000, fps: 120 });

  assert.equal(display.stats, '10m30s 120.0fps');
});

test('a finished build shows the stage alone, not its message', () => {
  assert.equal(progressDisplay({ ...PACKAGE, stage: 'done', message: 'Complete', percent: 100 }).stage, 'Done');
});

test('elapsed under a minute drops the minutes', () => {
  assert.equal(formatTime(9.7), '9s');
});

test('elapsed over a minute is minutes and seconds', () => {
  assert.equal(formatTime(125), '2m5s');
});
