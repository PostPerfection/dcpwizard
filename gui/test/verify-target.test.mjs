import assert from 'node:assert/strict';
import test from 'node:test';

import { PREVIEW_KIND_PACKAGE, PREVIEW_KIND_SOURCE } from '../../extern/guikit/src/preview-target.js';
import { verifyTarget } from '../src/verify-target.js';

test("Verify checks the recent package picked for preview", () => {
  const picked = { kind: PREVIEW_KIND_PACKAGE, path: "/dcps/picked" };
  assert.equal(verifyTarget({ selectedPreview: picked, openedPackage: "/dcps/opened", outputPath: "/dcps/built" }), "/dcps/picked");
});

test("with a source picked Verify checks the opened package, then the last build", () => {
  const source = { kind: PREVIEW_KIND_SOURCE, path: "/media/clip.mov" };
  assert.equal(verifyTarget({ selectedPreview: source, openedPackage: "/dcps/opened", outputPath: "/dcps/built" }), "/dcps/opened");
  assert.equal(verifyTarget({ selectedPreview: source, openedPackage: null, outputPath: "/dcps/built" }), "/dcps/built");
  assert.equal(verifyTarget({ selectedPreview: null, openedPackage: null, outputPath: null }), null);
});

test("a package the preview loaded comes before every other", () => {
  const picked = { kind: PREVIEW_KIND_PACKAGE, path: "/dcps/picked" };
  assert.equal(
    verifyTarget({ previewedPackage: "/dcps/playing", selectedPreview: picked, openedPackage: "/dcps/opened", outputPath: "/dcps/built" }),
    "/dcps/playing",
  );
});

test("with nothing else to check Verify checks the package beside the open project", () => {
  const source = { kind: PREVIEW_KIND_SOURCE, path: "/media/clip.mov" };
  assert.equal(
    verifyTarget({ previewedPackage: null, selectedPreview: source, openedPackage: null, outputPath: null, packageBesideProject: "/dcps/project" }),
    "/dcps/project",
  );
});

test("the package beside the open project loses to every other package", () => {
  const picked = { kind: PREVIEW_KIND_PACKAGE, path: "/dcps/picked" };
  const beside = { packageBesideProject: "/dcps/project" };
  assert.equal(verifyTarget({ ...beside, previewedPackage: "/dcps/playing" }), "/dcps/playing");
  assert.equal(verifyTarget({ ...beside, selectedPreview: picked }), "/dcps/picked");
  assert.equal(verifyTarget({ ...beside, openedPackage: "/dcps/opened" }), "/dcps/opened");
  assert.equal(verifyTarget({ ...beside, outputPath: "/dcps/built" }), "/dcps/built");
});
