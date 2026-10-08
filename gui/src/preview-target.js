export const PREVIEW_KIND_SOURCE = "source";
export const PREVIEW_KIND_PACKAGE = "package";

export function previewTarget({ selectedPreview, firstPicturePath, openedPackage, outputPath }) {
  if (selectedPreview?.kind === PREVIEW_KIND_SOURCE) return { kind: PREVIEW_KIND_SOURCE, path: selectedPreview.path };
  if (selectedPreview?.kind === PREVIEW_KIND_PACKAGE) return { kind: PREVIEW_KIND_PACKAGE, path: selectedPreview.path };
  if (firstPicturePath) return { kind: PREVIEW_KIND_SOURCE, path: firstPicturePath };
  if (openedPackage) return { kind: PREVIEW_KIND_PACKAGE, path: openedPackage };
  if (outputPath) return { kind: PREVIEW_KIND_PACKAGE, path: outputPath };
  return null;
}

// the package the Verify view checks when no package was chosen there
export function verifyTarget({ selectedPreview, openedPackage, outputPath }) {
  if (selectedPreview?.kind === PREVIEW_KIND_PACKAGE) return selectedPreview.path;
  return openedPackage ?? outputPath ?? null;
}

export function previewButtonEnabled(target, shownPath) {
  return target !== null && target.path !== shownPath;
}
