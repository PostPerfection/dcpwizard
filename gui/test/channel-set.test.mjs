import assert from 'node:assert/strict';
import test from 'node:test';

import { channelSetMeta, channelSetPreviewPath, mergeChannelSets, soundSource } from '../src/channel-set.js';

const STEM = '/media/260810 DST_MIX_V2.3.2';
const LANES = ['L', 'R', 'C', 'LFE', 'Ls', 'Rs'];

function looseAudio(id, lane) {
  return { id, type: 'audio', path: `${STEM}.${lane}.wav`, name: `260810 DST_MIX_V2.3.2.${lane}.wav`, meta: '' };
}

function groupOf(lanes) {
  return {
    prefix: '260810 DST_MIX_V2.3.2',
    files: lanes.map((lane, laneIndex) => ({ path: `${STEM}.${lane}.wav`, lane, laneIndex })),
  };
}

function idsFrom(first) {
  let next = first;
  return () => next++;
}

test('six loose stems become one set in lane order, in place of the first stem', () => {
  const picture = { id: 1, type: 'video', path: '/media/film.mov', name: 'film.mov', meta: '' };
  const stems = LANES.map((lane, index) => looseAudio(index + 2, lane));

  const { assets, absorbedInto } = mergeChannelSets([picture, ...stems], [groupOf(LANES)], idsFrom(8));

  assert.deepEqual(assets, [
    picture,
    {
      id: 8,
      type: 'audio',
      name: '260810 DST_MIX_V2.3.2',
      meta: '6 ch: L R C LFE Ls Rs',
      path: null,
      channelFiles: LANES.map((lane) => ({ path: `${STEM}.${lane}.wav`, lane })),
    },
  ]);
  assert.equal(absorbedInto.get(2), assets[1]);
  assert.equal(absorbedInto.size, 6);
});

test('a group of one file stays a plain audio asset', () => {
  const stem = looseAudio(1, 'L');

  const { assets, absorbedInto } = mergeChannelSets([stem], [groupOf(['L'])], idsFrom(2));

  assert.deepEqual(assets, [stem]);
  assert.equal(absorbedInto.size, 0);
});

test('a later stem grows the set it shares a name with', () => {
  const first = mergeChannelSets([looseAudio(1, 'L'), looseAudio(2, 'R')], [groupOf(['L', 'R'])], idsFrom(3));
  const withCentre = [...first.assets, looseAudio(4, 'C')];

  const { assets, absorbedInto } = mergeChannelSets(withCentre, [groupOf(['L', 'R', 'C'])], idsFrom(5));

  assert.equal(assets.length, 1);
  assert.equal(assets[0].meta, '3 ch: L R C');
  assert.equal(absorbedInto.get(3), assets[0]);
});

test('a set the groups give back unchanged is kept as it is', () => {
  const first = mergeChannelSets([looseAudio(1, 'L'), looseAudio(2, 'R')], [groupOf(['L', 'R'])], idsFrom(3));

  const { assets, absorbedInto } = mergeChannelSets(first.assets, [groupOf(['L', 'R'])], idsFrom(4));

  assert.deepEqual(assets, first.assets);
  assert.equal(absorbedInto.size, 0);
});

test('a set is sent as its files and previews nothing, a WAV as its path', () => {
  const set = mergeChannelSets([looseAudio(1, 'L'), looseAudio(2, 'R')], [groupOf(['L', 'R'])], idsFrom(3)).assets[0];
  const wav = looseAudio(4, 'C');

  assert.deepEqual(soundSource(set), { audioPath: null, audioChannelFiles: [`${STEM}.L.wav`, `${STEM}.R.wav`] });
  assert.deepEqual(soundSource(wav), { audioPath: `${STEM}.C.wav`, audioChannelFiles: null });
  assert.deepEqual(soundSource(null), { audioPath: null, audioChannelFiles: null });
  assert.equal(channelSetPreviewPath(set), null);
  assert.equal(channelSetPreviewPath(wav), wav.path);
  assert.equal(channelSetMeta(set.channelFiles), '2 ch: L R');
});
