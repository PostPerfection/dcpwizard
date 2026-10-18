import assert from 'node:assert/strict';
import test from 'node:test';

import { emptyTimeline } from '../../extern/guikit/src/composition-timeline.js';
import { clipSoundIsChannelSet, clipTrimSpecs, previewTimelineWire, timelineRequestFields, withChannelSets, withClipTrims } from '../src/composition-timeline.js';

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

function clip(picture, frames, inFrame, outFrame, { start = 0, layer = 0, sound = { kind: 'embedded', asset: null }, subtitle = null } = {}) {
  return { picture, frames, start, layer, opacity: 1, inFrame, outFrame, sound, subtitle };
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
      clip(2, 96, 10, 58, { start: 48, sound: { kind: 'file', asset: 4 }, subtitle: 5 }),
      clip(3, 24, 0, 24, { start: 96, sound: { kind: 'none', asset: null } }),
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
      version: 2,
      fps: { numerator: 24, denominator: 1 },
      clips: [
        { picture: '/media/a.mov', start: 0, layer: 0, opacity: 1, in_frame: 0, out_frame: 48, sound: 'Embedded', subtitle: null },
        { picture: '/media/b.mov', start: 48, layer: 0, opacity: 1, in_frame: 10, out_frame: 58, sound: { File: '/media/b.wav' }, subtitle: '/media/b.srt' },
        { picture: '/media/c', start: 96, layer: 0, opacity: 1, in_frame: 0, out_frame: 24, sound: 'None', subtitle: null },
      ],
      sound: 'PerClip',
      subtitle: null,
      reel_starts: [48],
    },
  });
  assert.deepEqual(requestFor({ ...timeline, sound: 6 }).timeline.sound, { Override: '/media/mix.wav' });
});

test('a channel set over a multi-clip timeline goes as the channel files beside a per-clip wire timeline', () => {
  const request = requestFor({ ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 48), clip(2, 96, 0, 96, { start: 48 })], sound: 7 });

  assert.deepEqual(request.audioChannelFiles, ['/media/mix_L.wav', '/media/mix_R.wav']);
  assert.equal(request.timeline.sound, 'PerClip');
});

test('a one-clip timeline the single-picture request cannot say goes as a timeline, the side its clip trims in the wire and a whole side\'s panel trim for submit_job to refuse', () => {
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 48, 240, { sound: { kind: 'none', asset: null } })], reelStarts: [96] };

  const request = requestFor(timeline, { ...PANEL_REQUEST, trimStart: '48f', trimEnd: '1s', splitAt: '00:00:04' });

  assert.equal(request.videoPath, '');
  assert.equal(request.trimStart, null);
  assert.equal(request.timeline.clips[0].in_frame, 48);
  assert.equal(request.trimEnd, '1s');
  assert.equal(request.splitAt, null);
  assert.deepEqual(request.timeline.reel_starts, [96]);
  assert.equal(request.timeline.clips[0].sound, 'None');
});

test('a one-clip timeline with black before its clip, or with its clip above layer 0, goes as a timeline', () => {
  const afterBlack = requestFor({ ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240, { start: 24 })] });
  const overBlack = requestFor({ ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240, { layer: 1, sound: { kind: 'none', asset: null } })] });

  assert.equal(afterBlack.videoPath, '');
  assert.equal(afterBlack.timeline.clips[0].start, 24);
  assert.equal(overBlack.videoPath, '');
  assert.equal(overBlack.timeline.clips[0].layer, 1);
});

test('the Trim fields set a one-clip timeline\'s in and out points, which the build and the preview both read', () => {
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240)], sound: 6 };

  const trimmed = withClipTrims(timeline, 48, 24);
  const trimSpecs = clipTrimSpecs(trimmed.clips[0]);
  const request = requestFor(trimmed, { ...PANEL_REQUEST, ...trimSpecs });
  const played = previewTimelineWire(trimmed, assetById).clips[0];

  assert.deepEqual(trimmed.clips[0], clip(1, 240, 48, 216));
  assert.deepEqual(trimSpecs, { trimStart: '48f', trimEnd: '24f' });
  assert.deepEqual([request.videoPath, request.trimStart, request.trimEnd], ['/media/a.mov', '48f', '24f']);
  assert.equal('timeline' in request, false);
  assert.deepEqual([played.in_frame, played.out_frame], [48, 216]);
  assert.deepEqual(clipTrimSpecs(timeline.clips[0]), { trimStart: '', trimEnd: '' });
  assert.throws(() => withClipTrims(timeline, 200, 40), RangeError);
  const unprobed = { ...timeline, clips: [clip(1, 0, 0, 0)] };
  assert.equal(withClipTrims(unprobed, 48, 24), unprobed, 'an unprobed clip has no end to trim from');
  const twoClips = { ...timeline, clips: [clip(1, 240, 0, 240), clip(2, 96, 0, 96, { start: 240 })] };
  assert.equal(withClipTrims(twoClips, 48, 24), twoClips);
});

test('a clip sound absorbed into a channel set moves to the timeline, which is the only place a set can go', () => {
  const set = assetById(7);
  const timeline = { ...emptyTimeline([24, 1]), clips: [clip(1, 240, 0, 240, { sound: { kind: 'file', asset: 6 } })] };

  const grouped = withChannelSets(timeline, new Map([[6, set]]));

  assert.deepEqual(grouped.clips[0].sound, { kind: 'embedded', asset: null });
  assert.equal(grouped.sound, 7);
  assert.equal(clipSoundIsChannelSet(grouped, assetById), false);
  assert.equal(clipSoundIsChannelSet({ ...timeline, clips: [clip(1, 240, 0, 240, { sound: { kind: 'file', asset: 7 } })] }, assetById), true);
  const overlay = clip(2, 96, 0, 96, { layer: 1, sound: { kind: 'none', asset: null } });
  const layered = withChannelSets({ ...timeline, clips: [...timeline.clips, overlay] }, new Map([[6, set]]));
  assert.deepEqual(layered.clips[1].sound, { kind: 'none', asset: null }, 'a clip above layer 0 stays soundless');
});

test('the preview plays a timeline a channel set sounds without sound, and any other with its own', () => {
  const clips = [clip(1, 240, 0, 48), clip(2, 96, 0, 96, { start: 48, sound: { kind: 'file', asset: 4 } })];

  const underChannelSet = previewTimelineWire({ ...emptyTimeline([24, 1]), clips, sound: 7 }, assetById);
  assert.equal(underChannelSet.sound, 'PerClip');
  assert.deepEqual(underChannelSet.clips.map((wired) => wired.sound), ['None', 'None']);

  const underWav = previewTimelineWire({ ...emptyTimeline([24, 1]), clips, sound: 6 }, assetById);
  assert.deepEqual(underWav.sound, { Override: '/media/mix.wav' });
  assert.deepEqual(underWav.clips.map((wired) => wired.sound), ['Embedded', { File: '/media/b.wav' }]);
});
