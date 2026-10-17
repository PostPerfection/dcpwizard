import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { register } from 'node:module';
import test from 'node:test';

import {
  FORM_CONTROLS,
  OUTPUT_FIELDS,
  TEXT_FIELDS,
  PROJECT_FILE_VERSION,
  PROJECT_FILE_MIGRATIONS,
  serializeForm,
  restoreFormState,
  audioMapCells,
  audioMapSpecFrom,
} from '../src/project-form.js';
import { withProbedFrames, withReelStartingClipsPlaced } from '../../extern/guikit/src/composition-timeline.js';

register('../../extern/guikit/test/tauri-plugins-hooks.mjs', import.meta.url);

const { readProjectFile } = await import('../../extern/guikit/src/project.js');

// the profile only fills other controls, the threshold only drives Auto-crop, the container view only changes the preview
const CONTROLS_NOT_SAVED = ['prop-profile', 'prop-auto-crop-threshold', 'prop-preview-in-container'];

const CONTROL_TAG = /<(?:input|select|textarea)\b[^>]*\bid="(prop-[^"]+)"/g;

function panelControls(fill) {
  const controls = new Map();
  FORM_CONTROLS.forEach(([, id, property], index) => {
    controls.set(id, { [property]: fill(id, property, index) });
  });
  return controls;
}

function emptyPanel(fill) {
  const controls = panelControls(fill);
  return {
    controls,
    elementById: (id) => controls.get(id),
    project: { title: '', assets: [], compositions: [], activeComposition: 0 },
    markerRows: [],
    ratings: [],
    joinedItems: { head: [], tail: [] },
  };
}

function editedPanel() {
  const panel = emptyPanel((id, property, index) => (property === 'checked' ? index % 2 === 0 : `${id} value`));
  const picture = { id: 3, type: 'video', path: '/media/film.mov', name: 'film.mov', meta: '1998×1080 24/1', width: 1998, height: 1080 };
  const sound = { id: 5, type: 'audio', path: '/media/mix.wav', name: 'mix.wav', meta: '' };
  const subtitle = { id: 6, type: 'subtitle', path: '/media/subs.xml', name: 'subs.xml', meta: '' };
  const trailerPicture = { id: 7, type: 'video', path: '/media/trailer.mov', name: 'trailer.mov', meta: '' };
  panel.project = {
    title: 'Film',
    assets: [picture, sound, subtitle, trailerPicture],
    compositions: [
      {
        id: 1,
        name: 'Main',
        contentKind: 'feature',
        timeline: {
          fps: [24, 1],
          clips: [
            { picture: picture.id, frames: 240, inFrame: 24, outFrame: 216, sound: { kind: 'file', asset: sound.id }, subtitle: subtitle.id },
            { picture: trailerPicture.id, frames: 96, inFrame: 0, outFrame: 96, sound: { kind: 'embedded', asset: null }, subtitle: null },
          ],
          sound: null,
          subtitle: null,
          reelStarts: [192],
        },
      },
      {
        id: 4,
        name: 'Teaser',
        contentKind: 'teaser',
        timeline: {
          fps: [24, 1],
          clips: [{ picture: trailerPicture.id, frames: 96, inFrame: 0, outFrame: 96, sound: { kind: 'embedded', asset: null }, subtitle: null }],
          sound: sound.id,
          subtitle: null,
          reelStarts: [],
        },
      },
    ],
    activeComposition: 1,
  };
  panel.markerRows.push({ id: 7, label: 'FFEC', position: '00:58:12:03' }, { id: 9, label: 'LFOC', position: '' });
  panel.ratings.push({ id: 2, agency: 'http://www.mpaa.org/2003-ratings', label: 'PG-13' });
  panel.joinedItems.head = ['Studio logo', 'Rating card'];
  panel.joinedItems.tail = ['End ident'];
  return panel;
}

function serialized(panel, audioMap = '1:L,2:R@-3') {
  return serializeForm({ ...panel, audioMap });
}

test('serialize then restore gives back every field of the build panel', () => {
  const edited = editedPanel();
  const saved = JSON.parse(JSON.stringify(serialized(edited)));
  const defaults = serialized(emptyPanel(() => ''), null);

  const reopened = emptyPanel(() => '');
  const restored = restoreFormState(saved, defaults, reopened);

  assert.deepEqual(serialized(reopened, restored.audioMap), serialized(edited));
  for (const [, id, property] of FORM_CONTROLS) {
    assert.deepEqual(reopened.controls.get(id)[property], edited.controls.get(id)[property], id);
  }
  assert.equal(restored.form.audioMap, '1:L,2:R@-3');
  assert.deepEqual(restored.notRestored, []);
});

test('marker and rating rows come back numbered from 1', () => {
  const reopened = emptyPanel(() => '');
  restoreFormState(serialized(editedPanel()), serialized(emptyPanel(() => ''), null), reopened);

  assert.deepEqual(reopened.markerRows, [
    { id: 1, label: 'FFEC', position: '00:58:12:03' },
    { id: 2, label: 'LFOC', position: '' },
  ]);
  assert.deepEqual(reopened.ratings.map((rating) => rating.id), [1]);
});

test('a field missing from the file takes the panel default', () => {
  const defaultsPanel = emptyPanel((id, property) => (property === 'checked' ? true : `${id} default`));
  const defaults = serialized(defaultsPanel, null);
  const reopened = editedPanel();

  restoreFormState({ title: 'Only a title' }, defaults, reopened);

  assert.equal(reopened.controls.get('prop-title').value, 'Only a title');
  assert.equal(reopened.controls.get('prop-standard').value, 'prop-standard default');
  assert.equal(reopened.controls.get('prop-encrypt').checked, true);
  assert.deepEqual(reopened.project.assets, []);
  assert.deepEqual(reopened.markerRows, []);
  assert.deepEqual(reopened.joinedItems, { head: [], tail: [] });
});

test('a version 1 project file opens and restores its saved fields', () => {
  const text = readFileSync(new URL('./fixtures/Film-version-1.dcpwizard', import.meta.url), 'utf8');
  const { form, version } = readProjectFile(text, 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const reopened = emptyPanel(() => '');

  const { notRestored } = restoreFormState(form, serialized(emptyPanel(() => ''), null), reopened);

  assert.equal(version, 1);
  assert.deepEqual(notRestored, []);
  assert.equal(reopened.controls.get('prop-title').value, 'Film');
  assert.equal(reopened.controls.get('prop-resolution').value, '2k-flat');
  assert.equal(reopened.controls.get('prop-audio-channels').value, '6');
  assert.equal(reopened.controls.get('prop-encrypt').checked, true);
  assert.equal(reopened.controls.get('prop-key-out').value, '/home/user/DCP/Film-keys.json');
  assert.equal(form.audioMap, '1:L,2:R@-3');
  const [picture, sound] = reopened.project.assets;
  assert.equal(picture.path, '/media/film.mov');
  assert.deepEqual(reopened.project.compositions[0].timeline, {
    fps: [24, 1],
    clips: [{ picture: picture.id, frames: 0, inFrame: 0, outFrame: 0, sound: { kind: 'file', asset: sound.id }, subtitle: null }],
    sound: null,
    subtitle: null,
    reelStarts: [],
  });
  assert.deepEqual(reopened.markerRows, [{ id: 1, label: 'FFEC', position: '00:58:12:03' }]);
  assert.deepEqual(reopened.ratings, [{ id: 1, agency: 'http://www.mpaa.org/2003-ratings', label: 'PG-13' }]);
  assert.deepEqual(reopened.joinedItems, { head: ['Studio logo'], tail: [] });
});

test('a version 1 project file, saved before the composition metadata fields, opens with them blank', () => {
  const text = readFileSync(new URL('./fixtures/Film-version-1.dcpwizard', import.meta.url), 'utf8');
  const { form } = readProjectFile(text, 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const reopened = editedPanel();

  restoreFormState(form, serialized(emptyPanel(() => ''), null), reopened);

  for (const id of ['prop-version-number', 'prop-chain', 'prop-distributor', 'prop-facility-name', 'prop-luminance']) {
    assert.equal(reopened.controls.get(id).value, '', id);
  }
});

test('a version 1 project file, saved before the subtitle placement fields, opens with them at the panel defaults', () => {
  const text = readFileSync(new URL('./fixtures/Film-version-1.dcpwizard', import.meta.url), 'utf8');
  const { form } = readProjectFile(text, 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const defaultsPanel = emptyPanel((id, property) => (property === 'checked' ? false : `${id} default`));
  const reopened = editedPanel();

  restoreFormState(form, serialized(defaultsPanel, null), reopened);

  for (const id of [
    'prop-subtitle-halign',
    'prop-subtitle-valign',
    'prop-subtitle-vposition',
    'prop-subtitle-zposition',
    'prop-subtitle-rtl',
    'prop-subtitle-wrap',
    'prop-subtitle-font',
  ]) {
    assert.equal(reopened.controls.get(id).value, `${id} default`, id);
  }
  assert.equal(reopened.controls.get('prop-subtitle-no-subset').checked, false);
});

test('a version 1 channel WAV directory is dropped on upgrade with a note saying so', () => {
  const saved = JSON.parse(readFileSync(new URL('./fixtures/Film-version-1.dcpwizard', import.meta.url), 'utf8'));
  saved.form.audioChannelDir = '/media/channels';
  const { form, upgradeNotes } = readProjectFile(JSON.stringify(saved), 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const reopened = emptyPanel(() => '');

  const { notRestored } = restoreFormState(form, serialized(emptyPanel(() => ''), null), reopened);

  assert.equal('audioChannelDir' in form, false);
  assert.deepEqual(upgradeNotes, ['Channel WAV directory /media/channels is no longer read: drop the WAVs on Assets instead']);
  assert.deepEqual(notRestored, []);
  assert.equal(reopened.controls.get('prop-title').value, 'Film');
});

test('a channel set as the timeline sound comes back with its files', () => {
  const panel = editedPanel();
  const set = {
    id: 8,
    type: 'audio',
    name: 'mix',
    meta: '2 ch: L R',
    path: null,
    channelFiles: [
      { path: '/media/mix_L.wav', lane: 'L' },
      { path: '/media/mix_R.wav', lane: 'R' },
    ],
  };
  panel.project.assets.push(set);
  panel.project.compositions[0].timeline.sound = set.id;
  const reopened = emptyPanel(() => '');

  restoreFormState(JSON.parse(JSON.stringify(serialized(panel))), serialized(emptyPanel(() => ''), null), reopened);

  assert.equal(reopened.project.compositions[0].timeline.sound, set.id);
  assert.deepEqual(reopened.project.assets.at(-1), set);
  assert.deepEqual(serialized(reopened), serialized(panel));
});

function selectOffering(label, values, value) {
  return { value, labels: [{ textContent: ` ${label} ` }], options: values.map((optionValue) => ({ value: optionValue })) };
}

test('a saved select value the panel does not offer is named and the select keeps its default', () => {
  const defaults = serialized(emptyPanel(() => ''), null);
  defaults.resolution = 'auto';
  const reopened = emptyPanel(() => '');
  reopened.controls.set('prop-resolution', selectOffering('Resolution', ['auto', '2k', '4k'], 'auto'));
  reopened.controls.set('prop-standard', selectOffering('Standard', ['smpte', 'interop'], 'smpte'));

  const { notRestored } = restoreFormState({ resolution: '8k', standard: 'interop' }, defaults, reopened);

  assert.deepEqual(notRestored, ['Resolution option 8k']);
  assert.equal(reopened.controls.get('prop-resolution').value, 'auto');
  assert.equal(reopened.controls.get('prop-standard').value, 'interop');
});

test('a timeline asset gone from the file is named by composition and place, and taken off the timeline', () => {
  const saved = JSON.parse(JSON.stringify(serialized(editedPanel())));
  saved.project.assets = saved.project.assets.filter((asset) => asset.id !== 5 && asset.id !== 7);
  const reopened = emptyPanel(() => '');

  const { notRestored } = restoreFormState(saved, serialized(emptyPanel(() => ''), null), reopened);

  assert.deepEqual(notRestored, ['Main clip 2 picture', 'Main clip 1 sound', 'Teaser clip 1 picture', 'Teaser timeline sound']);
  const [main, teaser] = reopened.project.compositions.map((composition) => composition.timeline);
  assert.deepEqual(main.clips.map((clip) => clip.picture), [3]);
  assert.deepEqual(main.clips[0].sound, { kind: 'embedded', asset: null });
  assert.equal(main.clips[0].subtitle, 6);
  assert.deepEqual(teaser.clips, []);
  assert.equal(teaser.sound, null);
});

function versionTwoProject(project) {
  return JSON.stringify({ wizard: 'dcpwizard', version: 2, saved: '2026-10-01T09:00:00.000Z', form: { title: 'Film', framerate: '25', project } });
}

test('a version 2 project opens with reel 1 as the first clip and the later reels appended, naming what moved', () => {
  const assets = [
    { id: 1, type: 'video', path: '/media/feature.mov', name: 'feature.mov', meta: '', fps: '24000/1001' },
    { id: 2, type: 'audio', path: '/media/feature.wav', name: 'feature.wav', meta: '' },
    { id: 3, type: 'subtitle', path: '/media/feature.srt', name: 'feature.srt', meta: '' },
    { id: 4, type: 'video', path: '/media/trailer.mov', name: 'trailer.mov', meta: '' },
    { id: 5, type: 'audio', path: null, name: 'mix', meta: '2 ch: L R', channelFiles: [{ path: '/media/mix_L.wav', lane: 'L' }, { path: '/media/mix_R.wav', lane: 'R' }] },
    { id: 6, type: 'subtitle', path: '/media/orphan.srt', name: 'orphan.srt', meta: '' },
  ];
  const text = versionTwoProject({
    title: 'Film',
    assets,
    compositions: [
      {
        id: 1,
        name: 'Main',
        contentKind: 'feature',
        reels: [
          { id: 1, picture: 1, sound: 2, subtitle: 3 },
          { id: 2, picture: 4, sound: 5, subtitle: null },
          { id: 3, picture: null, sound: null, subtitle: 6 },
        ],
      },
    ],
    activeComposition: 0,
  });

  const { form, upgradeNotes } = readProjectFile(text, 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const reopened = emptyPanel(() => '');
  const { notRestored } = restoreFormState(form, serialized(emptyPanel(() => ''), null), reopened);

  assert.equal(PROJECT_FILE_VERSION, 3);
  assert.deepEqual(reopened.project.compositions[0].timeline, {
    fps: [24000, 1001],
    clips: [
      { picture: 1, frames: 0, inFrame: 0, outFrame: 0, sound: { kind: 'file', asset: 2 }, subtitle: 3 },
      { picture: 4, frames: 0, inFrame: 0, outFrame: 0, sound: { kind: 'embedded', asset: null }, subtitle: null },
    ],
    sound: null,
    subtitle: null,
    reelStarts: [],
  });
  assert.deepEqual(upgradeNotes, [
    'Main reel 2 is clip 2 of the timeline',
    'Main reel 2 sound stays off the timeline: a channel set only sounds over the whole timeline',
    'Main reel 3 subtitle stays off the timeline: the reel has no picture',
  ]);
  assert.deepEqual(notRestored, []);
});

test('a version 2 project that built two reels starts a reel at its second clip once both clips are probed', () => {
  const assets = [
    { id: 1, type: 'video', path: '/media/feature.mov', name: 'feature.mov', meta: '', fps: '24/1' },
    { id: 2, type: 'video', path: '/media/trailer.mov', name: 'trailer.mov', meta: '', fps: '24/1' },
  ];
  const text = versionTwoProject({
    title: 'Film',
    assets,
    compositions: [
      {
        id: 1,
        name: 'Main',
        contentKind: 'feature',
        reels: [
          { id: 1, picture: 1, sound: null, subtitle: null },
          { id: 2, picture: 2, sound: null, subtitle: null },
        ],
      },
    ],
    activeComposition: 0,
  });
  const { form } = readProjectFile(text, 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const reopened = emptyPanel(() => '');
  restoreFormState(form, serialized(emptyPanel(() => ''), null), reopened);
  const probed = (assetId, frames) => {
    const [composition] = reopened.project.compositions;
    reopened.project.compositions = [withReelStartingClipsPlaced({ ...composition, timeline: withProbedFrames(composition.timeline, assetId, frames) })];
  };

  probed(1, 240);
  assert.deepEqual(reopened.project.compositions[0].timeline.reelStarts, [], 'the second clip has no start before its probe');
  assert.equal('reelStartingClipIndices' in serialized(reopened).project.compositions[0], false, 'the file never holds the clip list');
  probed(2, 96);

  const saved = serialized(reopened).project.compositions[0];
  assert.deepEqual(saved.timeline.reelStarts, [240]);
  assert.equal('reelStartingClipIndices' in saved, false);
  assert.equal('reelStartingClipIndices' in reopened.project.compositions[0], false);
});

test('a version 2 channel set on reel 1 becomes the timeline sound, and a project without assets opens empty', () => {
  const assets = [
    { id: 1, type: 'video', path: '/media/feature.mov', name: 'feature.mov', meta: '' },
    { id: 5, type: 'audio', path: null, name: 'mix', meta: '2 ch: L R', channelFiles: [{ path: '/media/mix_L.wav', lane: 'L' }, { path: '/media/mix_R.wav', lane: 'R' }] },
  ];
  const text = versionTwoProject({
    title: 'Film',
    assets,
    compositions: [{ id: 1, name: 'Main', contentKind: 'feature', reels: [{ id: 1, picture: 1, sound: 5, subtitle: null }] }],
    activeComposition: 0,
  });

  const { form } = readProjectFile(text, 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  const reopened = emptyPanel(() => '');
  const { notRestored } = restoreFormState(form, serialized(emptyPanel(() => ''), null), reopened);

  const { timeline } = reopened.project.compositions[0];
  assert.deepEqual(notRestored, []);
  assert.equal(timeline.sound, 5);
  assert.deepEqual(timeline.clips[0].sound, { kind: 'embedded', asset: null });
  assert.deepEqual(timeline.fps, [25, 1]);
  const titleOnly = readProjectFile(versionTwoProject(undefined), 'dcpwizard', PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS);
  assert.equal(titleOnly.form.project, undefined);
});

test('every build panel control is saved, and every saved control is on the panel', () => {
  const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');
  const onPanel = [...html.matchAll(CONTROL_TAG)].map((match) => match[1]);
  const saved = FORM_CONTROLS.map(([, id]) => id);

  assert.deepEqual(onPanel.filter((id) => !saved.includes(id) && !CONTROLS_NOT_SAVED.includes(id)), []);
  assert.deepEqual(saved.filter((id) => !onPanel.includes(id)), []);
});

function keysAtAnyDepth(value, keys = new Set()) {
  if (Array.isArray(value)) value.forEach((item) => keysAtAnyDepth(item, keys));
  else if (value && typeof value === 'object') {
    for (const [key, field] of Object.entries(value)) {
      keys.add(key);
      keysAtAnyDepth(field, keys);
    }
  }
  return keys;
}

test('the text and output field lists only name keys a saved form holds', () => {
  const saved = keysAtAnyDepth(serialized(editedPanel()));
  const topLevel = Object.keys(serialized(editedPanel()));

  assert.deepEqual(TEXT_FIELDS.filter((key) => !saved.has(key)), []);
  assert.deepEqual(OUTPUT_FIELDS.filter((key) => !topLevel.includes(key)), []);
});

test('the audio map spec comes back as the cells it was read from', () => {
  assert.deepEqual(audioMapCells('1:L,2:R@-3,3:C@0.5'), [
    { input: '1', lane: 'L', gain: '0' },
    { input: '2', lane: 'R', gain: '-3' },
    { input: '3', lane: 'C', gain: '0.5' },
  ]);
  assert.deepEqual(audioMapCells(null), []);
});

const UNTOUCHED_SET = [
  { typed: [], autoRoute: 'L' },
  { typed: [], autoRoute: 'R' },
  { typed: [], autoRoute: 'C' },
];

test('an untouched matrix sends no map', () => {
  assert.equal(audioMapSpecFrom(UNTOUCHED_SET), null);
});

test('one typed cell keeps the other rows on their automatic lanes', () => {
  const rows = UNTOUCHED_SET.map((row) => ({ ...row }));
  rows[1].typed = [{ lane: 'Rs', gain: '-3' }];

  assert.equal(audioMapSpecFrom(rows), '1:L,2:Rs@-3,3:C');
});

test('a typed row routes only its typed cells', () => {
  const rows = UNTOUCHED_SET.map((row) => ({ ...row }));
  rows[2].typed = [{ lane: 'C', gain: '0' }, { lane: 'Lc', gain: '-6' }];
  rows.push({ typed: [], autoRoute: null });

  assert.equal(audioMapSpecFrom(rows), '1:L,2:R,3:C,3:Lc@-6');
});
