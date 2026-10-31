import { PREVIEW_KIND_PACKAGE } from "../../extern/guikit/src/preview-target.js";

// the package the Verify view checks when none was chosen there, the open project's own package last
export function verifyTarget({ previewedPackage, selectedPreview, openedPackage, outputPath, packageBesideProject }) {
  if (previewedPackage) return previewedPackage;
  if (selectedPreview?.kind === PREVIEW_KIND_PACKAGE) return selectedPreview.path;
  return openedPackage ?? outputPath ?? packageBesideProject ?? null;
}
