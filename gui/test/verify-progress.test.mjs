import assert from 'node:assert/strict';
import test from 'node:test';

import { parseVerifyProgress, verifyProgressDisplay } from '../src/verify-progress.js';

test('a progress line from the shell plugin, newline and all, parses', () => {
  assert.deepEqual(parseVerifyProgress('progress hashes 1048576 20000000000\n'), {
    stage: 'hashes',
    done: 1048576,
    total: 20000000000,
  });
});

test('any other stderr line is left for the results', () => {
  assert.equal(parseVerifyProgress('ERROR hash mismatch for picture.mxf\n'), null);
  assert.equal(parseVerifyProgress('progress audio 1 2\n'), null);
  assert.equal(parseVerifyProgress('the progress hashes 1 2 line\n'), null);
});

test('the hash check shows its stage and whole percent', () => {
  assert.deepEqual(verifyProgressDisplay({ stage: 'hashes', done: 999, total: 1000 }), {
    percent: 99,
    text: 'Checking hashes 99%',
  });
});

test('the frame scan shows its stage and whole percent', () => {
  assert.deepEqual(verifyProgressDisplay({ stage: 'frames', done: 24, total: 48 }), {
    percent: 50,
    text: 'Scanning frames 50%',
  });
});

test('a stage with nothing to read is complete', () => {
  assert.equal(verifyProgressDisplay({ stage: 'frames', done: 0, total: 0 }).percent, 100);
});
