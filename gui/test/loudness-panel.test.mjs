import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseLoudnessTarget,
  gainToReachTarget,
  optionalNumber,
  sourceLine,
  deliveredLine,
  measurementSteps,
} from '../src/loudness-panel.js';

const levels = {
  integratedLufs: -27.34,
  leqMDb: 71.42,
  truePeakDbtp: -1.956,
  rangeLu: 19.21,
  shortTermMaxLufs: -15.2,
};

// the delivered levels carry a -3 dB gain the before levels exclude
const measurement = {
  before: levels,
  delivered: { ...levels, integratedLufs: levels.integratedLufs - 3, leqMDb: levels.leqMDb - 3 },
  steps: ['Routed the channel set by filename', 'Applied gain/fades'],
};

test('a lufs target sets the gain from the integrated loudness before the level step', () => {
  assert.equal(gainToReachTarget('lufs=-20', measurement), 7.3);
  assert.equal(gainToReachTarget('LUFS = -30', measurement), -2.7);
});

test('a leqm target sets the gain from the Leq(m) before the level step', () => {
  assert.equal(gainToReachTarget('leqm=85', measurement), 13.6);
  assert.equal(gainToReachTarget('Leq(m)=82', measurement), 10.6);
});

test('no usable target or no measurement leaves the button disabled', () => {
  assert.equal(gainToReachTarget('', measurement), null);
  assert.equal(gainToReachTarget('lufs=', measurement), null);
  assert.equal(gainToReachTarget('loud=3', measurement), null);
  assert.equal(gainToReachTarget('lufs=-20', null), null);
});

test('one press reaches the target and a second press after measuring again keeps it', () => {
  const gain = gainToReachTarget('lufs=-20', measurement);
  const remeasured = {
    before: levels,
    delivered: { ...levels, integratedLufs: levels.integratedLufs + gain },
    steps: [],
  };
  assert.ok(Math.abs(remeasured.delivered.integratedLufs + 20) < 0.05);
  assert.equal(gainToReachTarget('lufs=-20', remeasured), gain);
  assert.ok(Object.is(gainToReachTarget('lufs=-20', { ...measurement, before: { ...levels, integratedLufs: -20.04 } }), 0));
});

test('the target parses the way the build parses it', () => {
  assert.deepEqual(parseLoudnessTarget('leq=85'), { metric: 'leqm', value: 85 });
  assert.equal(parseLoudnessTarget('85'), null);
  assert.equal(parseLoudnessTarget('lufs=abc'), null);
});

test('a typed zero gain is a gain and an empty field is none', () => {
  assert.equal(optionalNumber('0'), 0);
  assert.equal(optionalNumber(' -6.5 '), -6.5);
  assert.equal(optionalNumber(''), null);
  assert.equal(optionalNumber(undefined), null);
});

test('the results lines read as the panel shows them', () => {
  assert.equal(
    sourceLine(measurement),
    'Source: Integrated -27.3 LUFS, Leq(m) 71.4 dB, true peak -1.96 dBTP, range 19.2 LU',
  );
  assert.equal(
    deliveredLine(measurement, true),
    'Delivered: Integrated -30.3 LUFS, Leq(m) 68.4 dB, true peak -1.96 dBTP, range 19.2 LU. Measure again to confirm',
  );
  assert.equal(measurementSteps(measurement), 'Routed the channel set by filename, Applied gain/fades');
  assert.equal(measurementSteps({ ...measurement, steps: [] }), 'Source track as is');
});
