import assert from 'node:assert/strict';
import test from 'node:test';

import { emptyTimeline } from '../../extern/guikit/src/composition-timeline.js';
import { containerViewRefusal } from '../src/container-view.js';

function clip(layer) {
  return { picture: 1, start: 0, layer, opacity: 1, inFrame: 0, outFrame: 48, sound: { kind: 'none', asset: null }, subtitle: null };
}

function timelineOf(clips) {
  return { ...emptyTimeline([24, 1]), clips };
}

test('the container view is off with a hint while a clip sits above layer 0', () => {
  const hint = containerViewRefusal(timelineOf([clip(0), clip(1)]), true);

  assert.match(hint, /above layer 0/);
});

test('the container view comes back when the upper clip moves down to layer 0', () => {
  assert.equal(containerViewRefusal(timelineOf([clip(0), clip(0)]), true), null);
});

test('the container view comes back when the upper clip goes', () => {
  assert.equal(containerViewRefusal(timelineOf([clip(0)]), true), null);
});

test('a clip that takes no picture filters keeps the container view off with its own hint', () => {
  const hint = containerViewRefusal(timelineOf([clip(0)]), false);

  assert.match(hint, /needs a video file source/);
});
