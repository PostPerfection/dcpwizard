import assert from 'node:assert/strict';
import test from 'node:test';

import { filledRatings, nextRatingRowId, ratingRowsFrom } from '../src/rating-rows.js';

test('only rows with both an agency and a label are sent, trimmed', () => {
  const rows = [
    { id: 1, agency: ' http://www.bbfc.co.uk ', label: ' 12A ' },
    { id: 2, agency: 'http://www.mpaa.org/2003-ratings', label: '  ' },
    { id: 3, agency: '', label: 'PG' },
  ];

  assert.deepEqual(filledRatings(rows), [{ agency: 'http://www.bbfc.co.uk', label: '12A' }]);
});

test('ratings read from a CPL become rows numbered from 1', () => {
  const rows = ratingRowsFrom([
    { agency: 'http://www.bbfc.co.uk', label: '12A' },
    { agency: 'http://www.mpaa.org/2003-ratings', label: 'PG-13' },
  ]);

  assert.deepEqual(rows.map((row) => row.id), [1, 2]);
  assert.equal(rows[1].label, 'PG-13');
});

test('a new row takes the id after the highest, even after a removal', () => {
  assert.equal(nextRatingRowId([]), 1);
  assert.equal(nextRatingRowId([{ id: 1 }, { id: 4 }]), 5);
});
