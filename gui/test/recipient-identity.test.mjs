import assert from 'node:assert/strict';
import test from 'node:test';

import { prefilledRecipientKey } from '../src/recipient-identity.js';

const CONFIGURED = { recipientKey: '/keys/recipient.key' };

test('a recipient key already chosen stays', () => {
  assert.equal(prefilledRecipientKey(CONFIGURED, '/keys/chosen.key'), '/keys/chosen.key');
});

test('an empty field takes the configured recipient key', () => {
  assert.equal(prefilledRecipientKey(CONFIGURED, ''), '/keys/recipient.key');
});

test('an empty field stays empty without a configured recipient key', () => {
  assert.equal(prefilledRecipientKey({ recipientKey: '' }, ''), '');
  assert.equal(prefilledRecipientKey({}, ''), '');
});
