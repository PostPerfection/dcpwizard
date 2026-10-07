import test from 'node:test';
import assert from 'node:assert/strict';
import { loudnessChartSvg } from '../src/loudness-chart.js';
import { parseLoudnessTarget } from '../src/loudness-panel.js';

// 31 values from 3.0 s to 6.0 s, so 3.0 s sits half way along the time axis
function levels(overrides = {}) {
  return {
    shortTermLufs: Array(31).fill(-30),
    shortTermFirstSeconds: 3,
    shortTermStepSeconds: 0.1,
    integratedLufs: -30,
    truePeakDbtp: -1.234,
    truePeakAtSeconds: 1.5,
    ...overrides,
  };
}

function curvePath(svg) {
  return svg.match(/class="loudness-chart-curve" d="([^"]*)"/)[1];
}

function texts(svg) {
  return [...svg.matchAll(/<text[^>]*>([^<]*)<\/text>/g)].map((match) => match[1]);
}

// the plot runs x 26 to 234 and y 14 (0 LUFS) to 122 (-60 LUFS)
test('a value maps to its time and level', () => {
  const svg = loudnessChartSvg(levels(), null);
  assert.match(curvePath(svg), /^M130 68 L/);
  assert.match(curvePath(svg), / L234 68$/);
  assert.match(svg, /class="loudness-chart-integrated" x1="26" y1="68" x2="234" y2="68"/);
  assert.match(svg, /class="loudness-chart-peak" x1="78" y1="14" x2="78" y2="122"/);
  assert.match(svg, /viewBox="0 0 250 140"/);
});

test('levels past the axis ends sit on the axis', () => {
  const svg = loudnessChartSvg(levels({ shortTermLufs: [-80, 3], shortTermStepSeconds: 3 }), null);
  assert.equal(curvePath(svg), 'M130 122 L234 14');
});

test('a silent window splits the curve', () => {
  const svg = loudnessChartSvg(levels({ shortTermLufs: [-20, -20, null, -20, -20] }), null);
  const path = curvePath(svg);
  assert.equal(path.match(/M/g).length, 2);
  assert.equal(path.match(/L/g).length, 2);
});

test('a lufs target draws its line and a Leq(m) target draws none', () => {
  const lufs = loudnessChartSvg(levels(), parseLoudnessTarget('lufs=-20'));
  assert.match(lufs, /class="loudness-chart-target" x1="26" y1="50" x2="234" y2="50"/);
  assert.ok(texts(lufs).includes('Target -20 LUFS'));
  const leqM = loudnessChartSvg(levels(), parseLoudnessTarget('leqm=85'));
  assert.doesNotMatch(leqM, /loudness-chart-target/);
  assert.doesNotMatch(loudnessChartSvg(levels(), null), /loudness-chart-target/);
});

test('the axes read in LUFS and mm:ss and the peak in dBTP', () => {
  const short = texts(loudnessChartSvg(levels(), null));
  for (const label of ['-60', '-50', '-40', '-30', '-20', '-10', '0', '00:00', '00:02', '00:04', '00:06', 'Integrated -30.0 LUFS', '-1.2 dBTP']) {
    assert.ok(short.includes(label), `no ${label} in ${short}`);
  }
  const tenMinutes = levels({ shortTermLufs: Array(5971).fill(-30) });
  const long = texts(loudnessChartSvg(tenMinutes, null));
  assert.deepEqual(long.filter((label) => label.includes(':')), ['00:00', '02:00', '04:00', '06:00', '08:00', '10:00']);
});

test('a silent track draws no integrated line and no peak mark', () => {
  const svg = loudnessChartSvg(levels({ shortTermLufs: [null, null], integratedLufs: null, truePeakDbtp: null }), null);
  assert.equal(curvePath(svg), '');
  assert.doesNotMatch(svg, /loudness-chart-integrated|loudness-chart-peak/);
});
