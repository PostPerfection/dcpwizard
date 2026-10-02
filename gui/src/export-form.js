// the first extension is what Save as suggests, an image sequence goes to a folder
const EXPORT_FORMATS = {
  prores: { extensions: ["mov"], takesCrf: false },
  h264: { extensions: ["mp4"], takesCrf: true },
  h265: { extensions: ["mp4"], takesCrf: true },
  dnxhr: { extensions: ["mov", "mxf"], takesCrf: false },
  "image-sequence": { extensions: [], takesCrf: false },
};

export const DEFAULT_CRF = 18;

export function takesCrf(format) {
  return EXPORT_FORMATS[format].takesCrf;
}

export function isMovieFormat(format) {
  return EXPORT_FORMATS[format].extensions.length > 0;
}

export function movieExtensions(format) {
  return EXPORT_FORMATS[format].extensions;
}

function extensionOf(path) {
  const name = path.split(/[/\\]/).pop();
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

// GTK's save dialog returns the typed name as is
export function withMovieExtension(path, format) {
  return extensionOf(path) ? path : `${path}.${movieExtensions(format)[0]}`;
}

function crfFrom(field) {
  const crf = Number.parseInt(field, 10);
  return Number.isNaN(crf) ? DEFAULT_CRF : crf;
}

export function exportRequestFrom(fields) {
  const refusals = [];
  if (!fields.input) refusals.push("Choose a DCP folder, CPL or picture MXF to export");
  if (!fields.output) refusals.push("Choose where to write the export");
  if (fields.kdm && !fields.recipientKey) refusals.push("A KDM needs the recipient private key it was issued to");
  if (fields.recipientKey && !fields.kdm) refusals.push("A recipient private key needs a KDM");

  const extensions = movieExtensions(fields.format);
  const extensionFits = !fields.output || extensions.length === 0 || extensions.includes(extensionOf(fields.output));
  if (!extensionFits) refusals.push(`The output file has to end in ${extensions.map((extension) => `.${extension}`).join(" or ")}`);

  if (refusals.length > 0) return { refusals };
  return {
    request: {
      input: fields.input,
      output: fields.output,
      format: fields.format,
      crf: takesCrf(fields.format) ? crfFrom(fields.crf) : null,
      audio: fields.audio || null,
      kdm: fields.kdm || null,
      recipient_key: fields.recipientKey || null,
      keys: fields.keys || null,
    },
  };
}

export function exportProgressText(progress) {
  return `${progress.frame} / ${progress.total_frames}, ${progress.fps.toFixed(1)} fps`;
}

export function exportProgressPercent(progress) {
  return progress.total_frames > 0 ? (100 * progress.frame) / progress.total_frames : 0;
}
