import assert from 'node:assert/strict';
import test from 'node:test';

import { compositionFrameAt, durationFrames, markerFromPlayerUnavailable, markerTimecode } from '../src/marker-from-player.js';

const UNTRIMMED = { durationSeconds: 600, sourceRate: '24/1', editRate: 24, trimStartFrames: 0, trimEndFrames: 0, padHeadFrames: 0 };

test('a source at the edit rate maps its frame straight onto the composition', () => {
  assert.equal(compositionFrameAt({ ...UNTRIMMED, positionSeconds: 2.5 }), 60);
});

test('a head trim moves the composition earlier and a head pad moves it later', () => {
  const frame = compositionFrameAt({ ...UNTRIMMED, positionSeconds: 1, trimStartFrames: 12, padHeadFrames: 48 });

  assert.equal(frame, 24 - 12 + 48);
});

test('a 23.976 source packaged at 24 counts its own frames, so an hour in stays on frame 86400', () => {
  const sourceFrame = 86400;
  const frame = compositionFrameAt({
    ...UNTRIMMED,
    durationSeconds: 7200,
    sourceRate: '24000/1001',
    positionSeconds: (sourceFrame * 1001) / 24000,
  });

  assert.equal(frame, sourceFrame);
});

test('a source at another rate is retimed onto the edit rate by its timestamps', () => {
  assert.equal(compositionFrameAt({ ...UNTRIMMED, sourceRate: '25/1', positionSeconds: 2 }), 48);
});

test('the end of the file is the last frame', () => {
  assert.equal(compositionFrameAt({ ...UNTRIMMED, durationSeconds: 4, positionSeconds: 4 }), 95);
});

test('a position in the trimmed head or tail is refused', () => {
  assert.throws(() => compositionFrameAt({ ...UNTRIMMED, positionSeconds: 0.25, trimStartFrames: 12 }), /trimmed head/);
  assert.throws(
    () => compositionFrameAt({ ...UNTRIMMED, durationSeconds: 4, positionSeconds: 3.75, trimEndFrames: 12 }),
    /trimmed tail/,
  );
});

test('a clip the player has not loaded is refused', () => {
  assert.throws(() => compositionFrameAt({ ...UNTRIMMED, durationSeconds: 0, positionSeconds: 0 }), /not loaded/);
});

test('the timecode reads back as the frame at the edit rate', () => {
  assert.equal(markerTimecode(0, 24), '00:00:00:00');
  assert.equal(markerTimecode(84000, 24), '00:58:20:00');
  assert.equal(markerTimecode(90061 * 25 + 7, 25), '25:01:01:07');
});

test('trim and pad fields count frames the way the build does', () => {
  assert.equal(durationFrames('', 24), 0);
  assert.equal(durationFrames('48f', 24), 48);
  assert.equal(durationFrames(' 2s ', 24), 48);
  assert.equal(durationFrames('1.5s', 24), 36);
  assert.throws(() => durationFrames('48', 24), /needs a unit/);
  assert.throws(() => durationFrames('1.01s', 24), /whole-frame/);
  assert.throws(() => durationFrames('xf', 24), /invalid frame count/);
});

test('the button says why it cannot read the player', () => {
  assert.match(markerFromPlayerUnavailable({ compositionPicturePath: undefined, shownPath: '/a.mov' }), /first reel/);
  assert.match(markerFromPlayerUnavailable({ compositionPicturePath: '/a.mov', shownPath: null }), /Nothing is playing/);
  assert.match(markerFromPlayerUnavailable({ compositionPicturePath: '/a.mov', shownPath: '/packages/Film' }), /different file/);
  assert.equal(markerFromPlayerUnavailable({ compositionPicturePath: '/a.mov', shownPath: '/a.mov' }), null);
});
