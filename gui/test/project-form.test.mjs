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
} from '../src/project-form.js';

register('../../extern/guikit/test/tauri-plugins-hooks.mjs', import.meta.url);

const { readProjectFile } = await import('../../extern/guikit/src/project.js');

// the profile only fills other controls, the threshold only drives Auto-crop
const CONTROLS_NOT_SAVED = ['prop-profile', 'prop-auto-crop-threshold'];

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
        reels: [
          { id: 1, picture, sound, subtitle },
          { id: 2, picture: null, sound: null, subtitle: null },
        ],
      },
      { id: 4, name: 'Teaser', contentKind: 'teaser', reels: [{ id: 1, picture: trailerPicture, sound: null, subtitle: null }] },
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

test('a restored reel holds the restored asset itself, so later edits to the asset reach it', () => {
  const reopened = emptyPanel(() => '');
  restoreFormState(JSON.parse(JSON.stringify(serialized(editedPanel()))), serialized(emptyPanel(() => ''), null), reopened);

  const [picture, sound] = reopened.project.assets;
  const firstReel = reopened.project.compositions[0].reels[0];
  assert.equal(firstReel.picture, picture);
  assert.equal(firstReel.sound, sound);
  assert.equal(reopened.project.compositions[1].reels[0].picture, reopened.project.assets[3]);
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
  assert.deepEqual(reopened.project.compositions[0].reels, [{ id: 1, picture, sound, subtitle: null }]);
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

test('a reel slot whose asset is gone from the file is named by composition and reel', () => {
  const saved = JSON.parse(JSON.stringify(serialized(editedPanel())));
  saved.project.assets = saved.project.assets.filter((asset) => asset.id !== 5);
  const reopened = emptyPanel(() => '');

  const { notRestored } = restoreFormState(saved, serialized(emptyPanel(() => ''), null), reopened);

  assert.deepEqual(notRestored, ['Main reel 1 sound']);
  assert.equal(reopened.project.compositions[0].reels[0].sound, null);
  assert.equal(reopened.project.compositions[0].reels[0].picture, reopened.project.assets[0]);
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
