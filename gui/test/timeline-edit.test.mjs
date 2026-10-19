import assert from 'node:assert/strict';
import { register } from 'node:module';
import test from 'node:test';

import { emptyTimeline } from '../../extern/guikit/src/composition-timeline.js';

register('../../extern/guikit/test/tauri-core-hooks.mjs', import.meta.url);
const bridge = await import('../../extern/guikit/test/tauri-core-stub.mjs');
const { acceptTimelineEdit } = await import('../src/timeline-edit.js');

const CANCEL_RENDER = 'preview_cancel_timeline_render';
const ASSETS = [
  { id: 1, type: 'video', path: '/media/a.mov', name: 'a.mov', fps: '25/1', frames: 240 },
  { id: 2, type: 'audio', path: null, name: 'mix', channelFiles: [{ path: '/media/mix_L.wav', lane: 'L' }, { path: '/media/mix_R.wav', lane: 'R' }] },
];

function assetById(assetId) {
  return ASSETS.find((asset) => asset.id === assetId);
}

function timelineWithClipSound(sound) {
  const clip = { picture: 1, start: 0, layer: 0, opacity: 1, inFrame: 0, outFrame: 48, sound, subtitle: null };
  return { ...emptyTimeline([24, 1]), clips: [clip] };
}

test('a timeline edit cancels the render of the timeline before it', () => {
  bridge.forgetInvocations();

  const stored = acceptTimelineEdit(timelineWithClipSound({ kind: 'embedded', asset: null }), assetById);

  assert.deepEqual(stored.fps, [25, 1]);
  assert.deepEqual(bridge.invocations.map(([command]) => command), [CANCEL_RENDER]);
});

test('a refused edit keeps the render going', () => {
  bridge.forgetInvocations();

  const stored = acceptTimelineEdit(timelineWithClipSound({ kind: 'file', asset: 2 }), assetById);

  assert.equal(stored, null);
  assert.deepEqual(bridge.invocations, []);
});
