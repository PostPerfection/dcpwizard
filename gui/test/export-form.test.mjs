import assert from 'node:assert/strict';
import test from 'node:test';

import {
  DEFAULT_CRF,
  exportRequestFrom,
  exportProgressText,
  exportProgressPercent,
  withMovieExtension,
  takesCrf,
  isMovieFormat,
} from '../src/export-form.js';

const PRORES = {
  input: '/films/Film_FTR',
  output: '/exports/film.mov',
  format: 'prores',
  crf: '18',
  audio: '',
  kdm: '',
  recipientKey: '',
  keys: '',
};

test('a filled form becomes the request the backend reads', () => {
  assert.deepEqual(exportRequestFrom(PRORES), {
    request: {
      input: '/films/Film_FTR',
      output: '/exports/film.mov',
      format: 'prores',
      crf: null,
      audio: null,
      kdm: null,
      recipient_key: null,
      keys: null,
    },
  });
});

test('an H.264 export carries its CRF and the key files', () => {
  const { request } = exportRequestFrom({
    ...PRORES,
    output: '/exports/film.mp4',
    format: 'h264',
    crf: '23',
    audio: '/films/sound.wav',
    kdm: '/keys/film.kdm.xml',
    recipientKey: '/keys/recipient.pem',
  });

  assert.equal(request.crf, 23);
  assert.equal(request.audio, '/films/sound.wav');
  assert.equal(request.kdm, '/keys/film.kdm.xml');
  assert.equal(request.recipient_key, '/keys/recipient.pem');
});

test('an empty CRF field falls back to the default', () => {
  assert.equal(exportRequestFrom({ ...PRORES, output: '/exports/film.mp4', format: 'h265', crf: '' }).request.crf, DEFAULT_CRF);
});

test('a form with no input and no output is refused for both', () => {
  assert.deepEqual(exportRequestFrom({ ...PRORES, input: '', output: '' }), {
    refusals: [
      'Choose a DCP folder, CPL or picture MXF to export',
      'Choose where to write the export',
    ],
  });
});

test('a KDM without the recipient key is refused', () => {
  assert.deepEqual(exportRequestFrom({ ...PRORES, kdm: '/keys/film.kdm.xml' }).refusals, [
    'A KDM needs the recipient private key it was issued to',
  ]);
});

test('a recipient key without a KDM is refused', () => {
  assert.deepEqual(exportRequestFrom({ ...PRORES, recipientKey: '/keys/recipient.pem' }).refusals, [
    'A recipient private key needs a KDM',
  ]);
});

test('a KEYS.json alone is enough for an encrypted export', () => {
  assert.equal(exportRequestFrom({ ...PRORES, keys: '/keys/KEYS.json' }).request.keys, '/keys/KEYS.json');
});

test('an output whose extension does not fit the format is refused', () => {
  assert.deepEqual(exportRequestFrom({ ...PRORES, output: '/exports/film.mp4' }).refusals, [
    'The output file has to end in .mov',
  ]);
  assert.deepEqual(exportRequestFrom({ ...PRORES, format: 'dnxhr', output: '/exports/film' }).refusals, [
    'The output file has to end in .mov or .mxf',
  ]);
});

test('the extension check ignores case and a dot in a folder name', () => {
  assert.ok(exportRequestFrom({ ...PRORES, output: '/exports/v1.2/FILM.MOV' }).request);
  assert.ok(exportRequestFrom({ ...PRORES, output: '/exports/v1.2/film' }).refusals);
});

test('a PNG sequence goes to a folder with any name', () => {
  const { request } = exportRequestFrom({ ...PRORES, format: 'image-sequence', output: '/exports/frames.v1' });

  assert.equal(request.output, '/exports/frames.v1');
  assert.equal(request.crf, null);
});

test('only the delivery codecs take a CRF and only an image sequence is not a movie', () => {
  assert.deepEqual(
    ['prores', 'h264', 'h265', 'dnxhr', 'image-sequence'].map((format) => [takesCrf(format), isMovieFormat(format)]),
    [[false, true], [true, true], [true, true], [false, true], [false, false]],
  );
});

test('a saved name without an extension gets the format default', () => {
  assert.equal(withMovieExtension('/exports/film', 'prores'), '/exports/film.mov');
  assert.equal(withMovieExtension('/exports/film', 'h264'), '/exports/film.mp4');
  assert.equal(withMovieExtension('/exports/film', 'dnxhr'), '/exports/film.mov');
  assert.equal(withMovieExtension('/exports/film.mxf', 'dnxhr'), '/exports/film.mxf');
});

test('the progress line shows frames written of the total and the rate', () => {
  const progress = { frame: 48, total_frames: 96, fps: 23.94, elapsed_secs: 2 };

  assert.equal(exportProgressText(progress), '48 / 96, 23.9 fps');
  assert.equal(exportProgressPercent(progress), 50);
});
