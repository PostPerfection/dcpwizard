import assert from 'node:assert/strict';
import test from 'node:test';

import { markerSpecs } from '../src/marker-specs.js';

test('each row with a position becomes a LABEL=position spec in row order', () => {
  const rows = [
    { label: 'FFEC', position: '00:58:12:03' },
    { label: 'FFMC', position: '84000' },
  ];

  assert.deepEqual(markerSpecs(rows), ['FFEC=00:58:12:03', 'FFMC=84000']);
});

test('a row with an empty position is skipped', () => {
  const rows = [
    { label: 'FFTC', position: '' },
    { label: 'LFTC', position: '   ' },
    { label: 'FFEC', position: '1200' },
  ];

  assert.deepEqual(markerSpecs(rows), ['FFEC=1200']);
});
