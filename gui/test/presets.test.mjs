import assert from 'node:assert/strict';
import { register } from 'node:module';
import test from 'node:test';

import { FORM_CONTROLS, PROJECT_ONLY_FORM_KEYS } from '../src/project-form.js';

register('../../extern/guikit/test/tauri-plugins-hooks.mjs', import.meta.url);

const { presetSnapshot, applyPresetForm, presetHint, appliedStatus, importStatus } = await import('../src/presets.js');

const AUDIO_MAP = '1:L,2:R@-3';

function panel(fill) {
  const controls = new Map();
  FORM_CONTROLS.forEach(([key, id, property], index) => {
    controls.set(id, { [property]: fill(key, property, index) });
  });
  return { controls, elementById: (id) => controls.get(id) };
}

function editedPanel() {
  return panel((key, property, index) => (property === 'checked' ? index % 2 === 0 : `${key} value`));
}

function blankPanel() {
  return panel((key, property) => (property === 'checked' ? false : ''));
}

function selectOffering(label, values, value) {
  return { value, labels: [{ textContent: ` ${label} ` }], options: values.map((optionValue) => ({ value: optionValue })) };
}

test('a snapshot holds every form key but the ones that belong to one title', () => {
  const { elementById } = editedPanel();

  const preset = presetSnapshot('Festival', elementById, AUDIO_MAP);

  const expectedKeys = FORM_CONTROLS.map(([key]) => key).filter((key) => !PROJECT_ONLY_FORM_KEYS.includes(key));
  assert.deepEqual(Object.keys(preset.form), expectedKeys);
  for (const key of PROJECT_ONLY_FORM_KEYS) assert.equal(key in preset.form, false, key);
  assert.equal(preset.form.standard, 'standard value');
  assert.equal(preset.name, 'Festival');
  assert.equal(preset.audioMap, AUDIO_MAP);
});

test('every key a preset leaves out is a form key', () => {
  const formKeys = FORM_CONTROLS.map(([key]) => key);

  assert.deepEqual(PROJECT_ONLY_FORM_KEYS.filter((key) => !formKeys.includes(key)), []);
});

test('applying a snapshot to a blank panel gives back every preset field and leaves the title alone', () => {
  const edited = editedPanel();
  const preset = JSON.parse(JSON.stringify(presetSnapshot('Festival', edited.elementById, AUDIO_MAP)));
  const reopened = blankPanel();

  const { applied, notApplied } = applyPresetForm(preset, reopened.elementById);

  assert.deepEqual(notApplied, []);
  assert.equal(applied.length, Object.keys(preset.form).length);
  for (const [key, id, property] of FORM_CONTROLS) {
    const expected = PROJECT_ONLY_FORM_KEYS.includes(key) ? (property === 'checked' ? false : '') : edited.controls.get(id)[property];
    assert.deepEqual(reopened.controls.get(id)[property], expected, id);
  }
});

test('a title or output folder written into a preset file is not applied', () => {
  const reopened = blankPanel();

  const { applied } = applyPresetForm({ form: { title: 'Other film', outputDir: '/elsewhere', standard: 'interop' } }, reopened.elementById);

  assert.deepEqual(applied, [reopened.controls.get('prop-standard')]);
  assert.equal(reopened.controls.get('prop-title').value, '');
  assert.equal(reopened.controls.get('prop-output').value, '');
});

test('a select value the panel does not offer is named and the select keeps its value', () => {
  const reopened = blankPanel();
  reopened.controls.set('prop-resolution', selectOffering('Resolution', ['auto', '2k-flat'], 'auto'));

  const { applied, notApplied } = applyPresetForm({ form: { resolution: '8k', bandwidth: '200' } }, reopened.elementById);

  assert.deepEqual(notApplied, ['Resolution option 8k']);
  assert.deepEqual(applied, [reopened.controls.get('prop-bandwidth')]);
  assert.equal(reopened.controls.get('prop-resolution').value, 'auto');
});

test('the hint counts the settings and the status names what was not applied', () => {
  assert.equal(presetHint('Festival', 74), 'Set by Festival: 74 settings. Edit any field to override.');
  assert.equal(appliedStatus('Festival', []), 'Preset Festival applied');
  assert.equal(appliedStatus('Festival', ['2 audio map routes']), 'Preset Festival applied, not applied: 2 audio map routes');
});

test('the import status names the presets it replaced', () => {
  assert.equal(importStatus({ imported: ['Festival', 'Advert'], replaced: ['Festival'] }), 'Imported 2 presets, replaced Festival');
  assert.equal(importStatus({ imported: ['Advert'], replaced: [] }), 'Imported 1 preset');
});
