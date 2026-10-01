// conform_source_to_dcp packages this source at 24 fps frame for frame, any other source is retimed onto the edit rate
const CONFORMED_SOURCE_RATE = "24000/1001";
const CONFORMED_EDIT_RATE = 24;

const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;
const TIMECODE_FIELD_DIGITS = 2;
// parse_pad_frames refuses seconds further than this from a whole frame
const WHOLE_FRAME_TOLERANCE = 1e-6;

export function markerFromPlayerUnavailable({ compositionPicturePath, shownPath }) {
  if (!compositionPicturePath) return "Put a video on the first reel to set a marker from the player";
  if (!shownPath) return "Nothing is playing in the preview";
  if (shownPath !== compositionPicturePath) return "The preview is playing a different file from the first reel's video";
  return null;
}

// the frames in a trim or pad field, refused the way parse_pad_frames refuses it
export function durationFrames(spec, editRate) {
  const text = spec.trim();
  if (!text) return 0;
  const number = text.slice(0, -1);
  const unit = text.slice(-1).toLowerCase();
  if (unit === "f") {
    if (!/^\+?\d+$/.test(number)) throw new Error(`invalid frame count in duration '${text}'`);
    return Number(number);
  }
  if (unit !== "s") throw new Error(`duration '${text}' needs a unit: frames (e.g. 48f) or seconds (e.g. 2s)`);
  const seconds = Number(number);
  if (number === "" || !Number.isFinite(seconds) || seconds < 0) throw new Error(`invalid seconds in duration '${text}'`);
  const exact = seconds * editRate;
  if (Math.abs(exact - Math.round(exact)) > WHOLE_FRAME_TOLERANCE) {
    throw new Error(`duration '${text}' is ${exact} frames at ${editRate} fps; use a whole-frame duration`);
  }
  return Math.round(exact);
}

function rateValue(rate) {
  const [numerator, denominator = "1"] = rate.split("/");
  return Number(numerator) / Number(denominator);
}

// the frame the build turns the source frame at positionSeconds into, counted in the composition
export function compositionFrameAt({ positionSeconds, durationSeconds, sourceRate, editRate, trimStartFrames, trimEndFrames, padHeadFrames }) {
  if (!(durationSeconds > 0)) throw new Error("The preview has not loaded the video yet");
  const conformed = sourceRate === CONFORMED_SOURCE_RATE && editRate === CONFORMED_EDIT_RATE;
  const readRate = conformed ? rateValue(sourceRate) : editRate;
  const sourceFrames = Math.round(durationSeconds * readRate);
  // at the end of the file the player reports the whole duration, one frame past the last
  const sourceFrame = Math.min(Math.round(positionSeconds * readRate), sourceFrames - 1);
  if (sourceFrame < trimStartFrames) throw new Error("The player is in the trimmed head, which the composition leaves out");
  if (sourceFrame >= sourceFrames - trimEndFrames) throw new Error("The player is in the trimmed tail, which the composition leaves out");
  return sourceFrame - trimStartFrames + padHeadFrames;
}

// the HH:MM:SS:FF parse_frame_offset reads back as the same frame
export function markerTimecode(frame, editRate) {
  const seconds = Math.floor(frame / editRate);
  const fields = [
    Math.floor(seconds / SECONDS_PER_HOUR),
    Math.floor((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE),
    seconds % SECONDS_PER_MINUTE,
    frame % editRate,
  ];
  return fields.map((field) => String(field).padStart(TIMECODE_FIELD_DIGITS, "0")).join(":");
}
