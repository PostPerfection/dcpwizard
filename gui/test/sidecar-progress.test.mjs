import assert from 'node:assert/strict';
import test from 'node:test';

import { parseSidecarProgress, sidecarProgressDisplay } from '../src/sidecar-progress.js';

test('a progress line from the shell plugin, newline and all, parses', () => {
  assert.deepEqual(parseSidecarProgress('progress hashes 1048576 20000000000\n'), {
    stage: 'hashes',
    done: 1048576,
    total: 20000000000,
  });
});

test('any other stderr line is left for the results', () => {
  assert.equal(parseSidecarProgress('ERROR hash mismatch for picture.mxf\n'), null);
  assert.equal(parseSidecarProgress('progress audio 1 2\n'), null);
  assert.equal(parseSidecarProgress('the progress hashes 1 2 line\n'), null);
});

test('the hash check shows its stage and whole percent', () => {
  assert.deepEqual(sidecarProgressDisplay({ stage: 'hashes', done: 999, total: 1000 }), {
    percent: 99,
    text: 'Checking hashes 99%',
  });
});

test('the frame scan shows its stage and whole percent', () => {
  assert.deepEqual(sidecarProgressDisplay({ stage: 'frames', done: 24, total: 48 }), {
    percent: 50,
    text: 'Scanning frames 50%',
  });
});

test('the drive copy shows its stage and whole percent', () => {
  assert.deepEqual(sidecarProgressDisplay(parseSidecarProgress('progress copy 1 4\n')), {
    percent: 25,
    text: 'Copying and verifying 25%',
  });
});

test('a stage with nothing to read is complete', () => {
  assert.equal(sidecarProgressDisplay({ stage: 'frames', done: 0, total: 0 }).percent, 100);
});
