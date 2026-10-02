import assert from 'node:assert/strict';
import test from 'node:test';

import { contentKeysFrom } from '../src/content-keys-form.js';

const EMPTY = { kdm: '', recipientKey: '', keys: '' };

test('a KDM and its recipient key become the keys the preview loads with', () => {
  assert.deepEqual(contentKeysFrom({ ...EMPTY, kdm: '/keys/film.kdm.xml', recipientKey: '/keys/recipient.pem' }), {
    contentKeys: { kdm: '/keys/film.kdm.xml', recipient_key: '/keys/recipient.pem', keys: null },
  });
});

test('a KEYS.json alone is enough', () => {
  assert.deepEqual(contentKeysFrom({ ...EMPTY, keys: '/films/KEYS.json' }), {
    contentKeys: { kdm: null, recipient_key: null, keys: '/films/KEYS.json' },
  });
});

test('an empty form is refused', () => {
  assert.deepEqual(contentKeysFrom(EMPTY), {
    refusals: ['Choose a KDM and its recipient private key, or a KEYS.json'],
  });
});

test('half a KDM pair is refused whichever half is missing', () => {
  assert.deepEqual(contentKeysFrom({ ...EMPTY, kdm: '/keys/film.kdm.xml' }), {
    refusals: ['A KDM needs the recipient private key it was issued to'],
  });
  assert.deepEqual(contentKeysFrom({ ...EMPTY, recipientKey: '/keys/recipient.pem', keys: '/films/KEYS.json' }), {
    refusals: ['A recipient private key needs a KDM'],
  });
});
