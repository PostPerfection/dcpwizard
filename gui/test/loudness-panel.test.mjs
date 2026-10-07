import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseLoudnessTarget,
  gainToReachTarget,
  measurementSummary,
  measurementSteps,
} from '../src/loudness-panel.js';

const measurement = {
  integratedLufs: -27.34,
  leqMDb: 71.42,
  truePeakDbtp: -1.956,
  rangeLu: 19.21,
  shortTermMaxLufs: -15.2,
  steps: ['Routed the channel set by filename', 'Applied gain/fades'],
};

test('a lufs target adds the distance from the integrated loudness to the gain', () => {
  assert.equal(gainToReachTarget('lufs=-20', measurement, ''), 7.3);
  assert.equal(gainToReachTarget('LUFS = -20', measurement, '-1.5'), 5.8);
});

test('a leqm target adds the distance from the Leq(m) to the gain', () => {
  assert.equal(gainToReachTarget('leqm=85', measurement, '0'), 13.6);
  assert.equal(gainToReachTarget('Leq(m)=82', measurement, '-2'), 8.6);
});

test('no usable target or no measurement leaves the button disabled', () => {
  assert.equal(gainToReachTarget('', measurement, '0'), null);
  assert.equal(gainToReachTarget('lufs=', measurement, '0'), null);
  assert.equal(gainToReachTarget('loud=3', measurement, '0'), null);
  assert.equal(gainToReachTarget('lufs=-20', null, '0'), null);
});

test('a second press after measuring again keeps the gain', () => {
  const gain = gainToReachTarget('lufs=-20', measurement, '0');
  const remeasured = { ...measurement, integratedLufs: measurement.integratedLufs + gain };
  const again = gainToReachTarget('lufs=-20', remeasured, String(gain));
  assert.equal(again, gain);
  assert.equal((again - gain).toFixed(1), '0.0');
  assert.ok(Object.is(gainToReachTarget('lufs=-20', { ...measurement, integratedLufs: -20.04 }, '0'), 0));
});

test('the target parses the way the build parses it', () => {
  assert.deepEqual(parseLoudnessTarget('leq=85'), { metric: 'leqm', value: 85 });
  assert.equal(parseLoudnessTarget('85'), null);
  assert.equal(parseLoudnessTarget('lufs=abc'), null);
});

test('the results lines read as the panel shows them', () => {
  assert.equal(
    measurementSummary(measurement),
    'Integrated -27.3 LUFS, Leq(m) 71.4 dB, true peak -1.96 dBTP, range 19.2 LU',
  );
  assert.equal(measurementSteps(measurement), 'Routed the channel set by filename, Applied gain/fades');
  assert.equal(measurementSteps({ ...measurement, steps: [] }), 'Source track as is');
});
