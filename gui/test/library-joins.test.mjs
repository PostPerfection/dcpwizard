import assert from 'node:assert/strict';
import test from 'node:test';

import { dropIntoJoin, joinedPayload, libraryPayload } from '../src/library-joins.js';

test('a library item dropped on a run goes to its end', () => {
  const joinedItems = { head: ['Logo'], tail: [] };

  dropIntoJoin(joinedItems, libraryPayload('Rating'), 'head', null);

  assert.deepEqual(joinedItems, { head: ['Logo', 'Rating'], tail: [] });
});

test('a library item dropped on a chip goes in front of it', () => {
  const joinedItems = { head: ['Logo', 'Rating'], tail: [] };

  dropIntoJoin(joinedItems, libraryPayload('Sponsor'), 'head', 1);

  assert.deepEqual(joinedItems.head, ['Logo', 'Sponsor', 'Rating']);
});

test('a joined item dragged to the other run leaves the first', () => {
  const joinedItems = { head: ['Logo', 'Rating'], tail: ['Credits'] };

  dropIntoJoin(joinedItems, joinedPayload('head', 0), 'tail', null);

  assert.deepEqual(joinedItems, { head: ['Rating'], tail: ['Credits', 'Logo'] });
});

test('a joined item dragged onto an earlier chip moves in front of it', () => {
  const joinedItems = { head: ['Logo', 'Rating', 'Sponsor'], tail: [] };

  dropIntoJoin(joinedItems, joinedPayload('head', 2), 'head', 0);

  assert.deepEqual(joinedItems.head, ['Sponsor', 'Logo', 'Rating']);
});

test('a joined item dragged onto a later chip lands in front of that chip', () => {
  const joinedItems = { head: ['Logo', 'Rating', 'Sponsor'], tail: [] };

  dropIntoJoin(joinedItems, joinedPayload('head', 0), 'head', 2);

  assert.deepEqual(joinedItems.head, ['Rating', 'Logo', 'Sponsor']);
});

test('a drop that is neither a library item nor a joined item changes nothing', () => {
  const joinedItems = { head: ['Logo'], tail: [] };

  dropIntoJoin(joinedItems, '/home/u/clip.mov', 'head', null);

  assert.deepEqual(joinedItems, { head: ['Logo'], tail: [] });
});
