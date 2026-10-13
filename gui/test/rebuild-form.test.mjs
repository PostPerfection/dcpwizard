import assert from 'node:assert/strict';
import test from 'node:test';

import {
  CUSTOM_CONTAINER,
  KEEP_CONTAINER,
  frameRateText,
  rebuildProgressText,
  rebuildReady,
  rebuildRequestFrom,
  sourceSummary,
} from '../src/rebuild-form.js';

const FIELDS = {
  source: '/films/Film_FTR',
  outputParent: '/films/rebuilt',
  title: ' Film UK ',
  container: KEEP_CONTAINER,
  containerDims: '',
  bitRate: '',
};
const JOINED = { head: ['Studio Ident'], tail: ['Rating Card'] };
const RATING_ROWS = [
  { id: 1, agency: 'http://www.bbfc.co.uk', label: '12A' },
  { id: 2, agency: '', label: '' },
];

test('a filled form becomes the request the backend reads', () => {
  assert.deepEqual(rebuildRequestFrom(FIELDS, JOINED, RATING_ROWS), {
    request: {
      source: '/films/Film_FTR',
      output_parent: '/films/rebuilt',
      title: 'Film UK',
      head_items: ['Studio Ident'],
      tail_items: ['Rating Card'],
      ratings: [{ agency: 'http://www.bbfc.co.uk', label: '12A' }],
      container: KEEP_CONTAINER,
      container_dims: null,
      video_bit_rate_mbps: null,
    },
  });
});

test('an emptied rating list sends no ratings, which removes the source ones', () => {
  assert.deepEqual(rebuildRequestFrom(FIELDS, JOINED, []).request.ratings, []);
});

test('a named container and a bit rate reach the request', () => {
  const { request } = rebuildRequestFrom({ ...FIELDS, container: '2k-scope', bitRate: '150' }, JOINED, RATING_ROWS);

  assert.equal(request.container, '2k-scope');
  assert.equal(request.container_dims, null);
  assert.equal(request.video_bit_rate_mbps, 150);
});

test('a custom container carries its size and needs one written as WxH', () => {
  const custom = { ...FIELDS, container: CUSTOM_CONTAINER };

  assert.equal(rebuildRequestFrom({ ...custom, containerDims: ' 1920x1080 ' }, JOINED, []).request.container_dims, '1920x1080');
  assert.deepEqual(rebuildRequestFrom({ ...custom, containerDims: '1920 by 1080' }, JOINED, []).refusals, [
    'Give the custom container as WxH, such as 1920x1080',
  ]);
});

test('a missing source, folder or title and a bad bit rate are each named', () => {
  const { refusals } = rebuildRequestFrom({ ...FIELDS, source: '', outputParent: '', title: '  ', bitRate: '12.5' }, JOINED, []);

  assert.deepEqual(refusals, [
    'Choose the DCP to rebuild',
    'Choose the folder the new DCP is written into',
    'Give the new DCP a title',
    'The bit rate has to be a whole number of Mbit/s above 0',
  ]);
});

test('Rebuild waits for a readable source, a folder and a title', () => {
  assert.equal(rebuildReady(FIELDS, true), true);
  assert.equal(rebuildReady(FIELDS, false), false);
  assert.equal(rebuildReady({ ...FIELDS, outputParent: '' }, true), false);
  assert.equal(rebuildReady({ ...FIELDS, title: ' ' }, true), false);
});

test('the source summary names what the CPL and the picture track declare', () => {
  const summary = sourceSummary({
    title: 'Film',
    standard: 'Smpte',
    stored_width: 2048,
    stored_height: 1080,
    active_width: 1998,
    active_height: 1080,
    edit_rate_numerator: 24,
    edit_rate_denominator: 1,
    reel_count: 2,
    total_frames: 2400,
    content_kind: '',
    ratings: [],
  });

  assert.equal(
    summary,
    [
      'Title: Film',
      'Standard: SMPTE',
      'Picture: 2048×1080, active area 1998×1080',
      'Frame rate: 24 fps',
      'Reels: 2, 2400 frames',
      'Kind: feature',
    ].join('\n'),
  );
});

test('a fractional edit rate is written to three decimals', () => {
  assert.equal(frameRateText(24000, 1001), '23.976 fps');
});

test('the progress line counts frames only while there are frames to count', () => {
  assert.equal(
    rebuildProgressText({ percent: 40, message: 're-encoding the picture of reel 1 of 1', frame: 12, total_frames: 24 }),
    're-encoding the picture of reel 1 of 1, frame 12 / 24',
  );
  assert.equal(rebuildProgressText({ percent: 95, message: 'moving picture.mxf', frame: 0, total_frames: 0 }), 'moving picture.mxf');
});
