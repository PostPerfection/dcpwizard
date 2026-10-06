import { joinPath } from "./kdm-form.js";

export const SUBTITLE_CONVERSION_EXTENSION = "xml";
export const BURN_IN_EXTENSION = "mp4";
export const BURN_IN_NAME_PART = "burnin";
export const TARGET_CONVERSION_EXTENSION = "mov";

const NAME_PART_SEPARATOR = "_";

function parentFolder(path) {
  return path.replace(/[/\\][^/\\]*$/, "");
}

function stem(path) {
  return path.split(/[/\\]/).pop().replace(/\.[^.]*$/, "");
}

export function outputFileInFolder({ folder, input, nameParts = [], extension }) {
  const name = [stem(input), ...nameParts].join(NAME_PART_SEPARATOR);
  return joinPath(folder || parentFolder(input), `${name}.${extension}`);
}

const QC_REPORT_ENDING = ".qc.html";

// Validate flags any file inside the package that it does not list
export function qcReportPathBeside(packageDirectory) {
  return `${packageDirectory.replace(/[/\\]+$/, "")}${QC_REPORT_ENDING}`;
}
