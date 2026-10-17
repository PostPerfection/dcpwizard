import assert from 'node:assert/strict';
import test from 'node:test';

import {
  clipSoundIsChannelSet,
  emptyTimeline,
  reelSpans,
  sourceFrameCount,
  splitTimecodeFrames,
  splitTimecodeSpec,
  timelineFrameAtSourceFrame,
  timelineRequestFields,
  withChannelSets,
  withFirstClipRate,
  withImportedAsset,
  withProbedFrames,
  withUnplacedAssetsFilled,
} from '../src/composition-timeline.js';

const ASSETS = [
  { id: 1, type: 'video', path: '/media/a.mov', name: 'a.mov', fps: '24/1', frames: 240 },
  { id: 2, type: 'video', path: '/media/b.mov', name: 'b.mov', fps: '24/1', frames: 96 },
  { id: 3, type: 'video', path: '/media/c', name: 'c', frames: 24 },
  { id: 4, type: 'audio', path: '/media/b.wav', name: 'b.wav' },
  { id: 5, type: 'subtitle', path: '/media/b.srt', name: 'b.srt' },
  { id: 6, type: 'audio', path: '/media/mix.wav', name: 'mix.wav' },
  { id: 7, type: 'audio', path: null, name: 'mix', channelFiles: [{ path: '/media/mix_L.wav', lane: 'L' }, { path: '/media/mix_R.wav', lane: 'R' }] },
  { id: 8, type: 'subtitle', path: '/media/feature.srt', name: 'feature.srt' },
];

function assetById(assetId) {
  return ASSETS.find((asset) => asset.id === assetId);
}

// what jobRequest reads off the panel, with the fields a timeline decides filled in to show they are replaced
const PANEL_REQUEST = {
  videoPath: '',
  title: 'Film',
  audioPath: null,
  subtitle: null,
  audioChannelFiles: null,
  trimStart: null,
  trimEnd: null,
  stillLength: null,
  reelLengthMinutes: 0,
  splitAt: null,
  splitChapters: false,
};

function requestFor(timeline, panel = PANEL_REQUEST) {
  return { ...panel, ...timelineRequestFields(timeline, assetById, panel) };
}

function clip(picture, frames, inFrame, outFrame, sound = { kind: 'embedded', asset: null }, subtitle = null) {
  return { picture, frames, inFrame, outFrame, sound, subtitle };
}

test('a one-clip timeline trimmed by its in and out points sends the single-picture request', () => {
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 48, 216)], sound: 6, subtitle: 8 };

  assert.deepEqual(requestFor({ ...timeline, subtitle: null }), {
    ...PANEL_REQUEST,
    videoPath: '/media/a.mov',
    audioPath: '/media/mix.wav',
    audioChannelFiles: null,
    subtitle: null,
    trimStart: '48f',
    trimEnd: '24f',
  });
  // the trim fields still trim a clip that keeps its whole source
  const untrimmed = requestFor({ ...timeline, clips: [clip(1, 240, 0, 240)] }, { ...PANEL_REQUEST, trimStart: '2s', trimEnd: '1s' });
  assert.equal(untrimmed.subtitle, '/media/feature.srt');
  assert.equal(untrimmed.trimStart, '2s');
  assert.equal(untrimmed.trimEnd, '1s');
  assert.equal('timeline' in untrimmed, false);
});

test('a channel set covering a one-clip timeline goes as the channel files', () => {
  const request = requestFor({ ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240)], sound: 7 });

  assert.equal(request.audioPath, null);
  assert.deepEqual(request.audioChannelFiles, ['/media/mix_L.wav', '/media/mix_R.wav']);
});

test('a multi-clip timeline sends postkit\'s timeline with paths and none of the fields it replaces', () => {
  const timeline = {
    fps: [24, 1],
    clips: [
      clip(1, 240, 0, 48),
      clip(2, 96, 10, 58, { kind: 'file', asset: 4 }, 5),
      clip(3, 24, 0, 24, { kind: 'none', asset: null }),
    ],
    sound: null,
    subtitle: null,
    reelStarts: [48],
  };
  const filledPanel = {
    ...PANEL_REQUEST,
    trimStart: '48f',
    trimEnd: '2s',
    stillLength: '5s',
    reelLengthMinutes: 10,
    splitAt: '00:00:02',
    splitChapters: true,
  };

  assert.deepEqual(requestFor(timeline, filledPanel), {
    ...PANEL_REQUEST,
    timeline: {
      version: 1,
      fps: { numerator: 24, denominator: 1 },
      clips: [
        { picture: '/media/a.mov', in_frame: 0, out_frame: 48, sound: 'Embedded', subtitle: null },
        { picture: '/media/b.mov', in_frame: 10, out_frame: 58, sound: { File: '/media/b.wav' }, subtitle: '/media/b.srt' },
        { picture: '/media/c', in_frame: 0, out_frame: 24, sound: 'None', subtitle: null },
      ],
      sound: 'PerClip',
      subtitle: null,
      reel_starts: [48],
    },
  });
  assert.deepEqual(requestFor({ ...timeline, sound: 6 }).timeline.sound, { Override: '/media/mix.wav' });
});

test('a one-clip timeline the single-picture request cannot say goes as a timeline, with the panel trim for submit_job to refuse', () => {
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 48, 240)], reelStarts: [96] };

  const request = requestFor(timeline, { ...PANEL_REQUEST, trimStart: '2s', splitAt: '00:00:04' });

  assert.equal(request.videoPath, '');
  assert.equal(request.trimStart, '2s');
  assert.equal(request.splitAt, null);
  assert.deepEqual(request.timeline.reel_starts, [96]);
  assert.equal(requestFor({ ...timeline, clips: [clip(1, 240, 0, 240, { kind: 'none', asset: null })] }).timeline.clips[0].sound, 'None');
});

test('an imported video becomes the last clip and the first sound and subtitle cover the timeline', () => {
  let timeline = emptyTimeline([24, 1]);
  for (const assetId of [1, 6, 8, 2, 4, 5]) timeline = withImportedAsset(timeline, assetById(assetId));

  assert.deepEqual(timeline, {
    fps: [24, 1],
    clips: [clip(1, 240, 0, 240), clip(2, 96, 0, 96)],
    sound: 6,
    subtitle: 8,
    reelStarts: [],
  });
});

test('an imported sound stays off the timeline while a clip has a sound file of its own', () => {
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240, { kind: 'file', asset: 4 }, 5)] };

  assert.deepEqual(withImportedAsset(withImportedAsset(timeline, assetById(6)), assetById(8)), timeline);
});

test('an unprobed clip takes its frame count, and the first clip its rate', () => {
  const unprobed = withImportedAsset(emptyTimeline([24, 1]), { ...assetById(2), frames: undefined });
  const probed = withProbedFrames(unprobed, 2, 96);

  assert.deepEqual(unprobed.clips[0], clip(2, 0, 0, 0));
  assert.deepEqual(probed.clips[0], clip(2, 96, 0, 96));
  const ntsc = (assetId) => ({ ...assetById(assetId), fps: '24000/1001' });
  assert.deepEqual(withFirstClipRate(probed, ntsc).fps, [24000, 1001]);
  assert.equal(withFirstClipRate(probed, assetById), probed);
  assert.equal(sourceFrameCount(4.004, '24000/1001'), 96);
});

test('filling the timeline places the first unplaced video only on an empty timeline', () => {
  const filled = withUnplacedAssetsFilled(emptyTimeline([24, 1]), ASSETS);

  assert.deepEqual(filled.clips.map((placed) => placed.picture), [1]);
  assert.equal(filled.sound, 4);
  assert.equal(filled.subtitle, 5);
  assert.deepEqual(withUnplacedAssetsFilled(filled, ASSETS), filled);
});

test('a clip sound absorbed into a channel set moves to the timeline, which is the only place a set can go', () => {
  const set = assetById(7);
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240, { kind: 'file', asset: 6 })] };

  const grouped = withChannelSets(timeline, new Map([[6, set]]));

  assert.deepEqual(grouped.clips[0].sound, { kind: 'embedded', asset: null });
  assert.equal(grouped.sound, 7);
  assert.equal(clipSoundIsChannelSet(grouped, assetById), false);
  assert.equal(clipSoundIsChannelSet({ ...timeline, clips: [clip(1, 240, 0, 240, { kind: 'file', asset: 7 })] }, assetById), true);
});

test('split timecodes and reel starts read back as the same frames', () => {
  assert.deepEqual(splitTimecodeFrames('00:00:02, 00:01:00:12', 24), [48, 1452]);
  assert.deepEqual(splitTimecodeFrames('', 24), []);
  assert.equal(splitTimecodeFrames('00:00', 24), null);
  assert.equal(splitTimecodeFrames('00:00:01:24', 24), null);
  assert.equal(splitTimecodeSpec([48, 1452], 24), '00:00:02:00, 00:01:00:12');
});

test('each reel span is named by the clip it starts in', () => {
  const timeline = {
    fps: [24, 1],
    clips: [clip(1, 240, 0, 96), clip(2, 96, 12, 84, { kind: 'file', asset: 4 }, 5)],
    sound: null,
    subtitle: null,
    reelStarts: [96, 500],
  };

  assert.deepEqual(reelSpans(timeline, assetById), [
    { frames: 96, picturePath: '/media/a.mov', soundPath: '', subtitlePath: '' },
    { frames: 72, picturePath: '/media/b.mov', soundPath: '/media/b.wav', subtitlePath: '/media/b.srt' },
  ]);
  assert.deepEqual(reelSpans(emptyTimeline([24, 1]), assetById), []);
});

test('a source frame of the second clip lands after the first clip on the timeline', () => {
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 96), clip(2, 96, 12, 84)] };

  assert.equal(timelineFrameAtSourceFrame(timeline, 1, 12), 96);
  assert.equal(timelineFrameAtSourceFrame(timeline, 1, 50), 134);
});
