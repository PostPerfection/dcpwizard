export const COLOUR_NOT_DECLARED = "colour space not declared";

const BITS_PER_MEGABIT = 1_000_000;
const FRAME_RATE_DECIMALS = 3;
const BIT_RATE_DECIMALS = 1;

const UNDECLARED_COLOUR_TAGS = new Set(["unknown", "unspecified", "reserved"]);

// the p3 option is the DCI white point, not P3-D65
const SOURCE_COLOURSPACE_BY_PRIMARIES = { bt709: "rec709", bt2020: "rec2020", smpte431: "p3", smpte428: "xyz" };
const SOURCE_COLOURSPACE_BY_MATRIX = { bt709: "rec709", bt2020nc: "rec2020", bt2020c: "rec2020" };
const SOURCE_COLOURSPACE_BY_TRANSFER = {
  bt709: "rec709",
  "bt2020-10": "rec2020",
  "bt2020-12": "rec2020",
  smpte428: "xyz",
};

// a ProRes clip can carry the matrix alone
const COLOUR_TAGS_BY_PRIORITY = [
  ["colorPrimaries", SOURCE_COLOURSPACE_BY_PRIMARIES],
  ["colorSpace", SOURCE_COLOURSPACE_BY_MATRIX],
  ["colorTransfer", SOURCE_COLOURSPACE_BY_TRANSFER],
];

function declared(tag) {
  return Boolean(tag) && !UNDECLARED_COLOUR_TAGS.has(tag);
}

function detectedColour(probe) {
  const decidingTag = COLOUR_TAGS_BY_PRIORITY.find(([key]) => declared(probe[key]));
  if (!decidingTag) return { sourceColourspace: null, label: COLOUR_NOT_DECLARED };
  const [key, colourspaceByTag] = decidingTag;
  const tag = probe[key];
  const sourceColourspace = colourspaceByTag[tag] ?? null;
  return { sourceColourspace, label: sourceColourspace ? tag : `${tag}, no matching colour space` };
}

function framesPerSecond(rate) {
  const [numerator, denominator] = String(rate ?? "").split("/").map(Number);
  if (!numerator || !denominator) return null;
  return numerator / denominator;
}

// 23.976 matches no option
function detectedFramerate(rate, framerateOptions) {
  const match = /^(\d+)\/1$/.exec(rate ?? "");
  if (!match) return null;
  return framerateOptions.find((option) => parseInt(option) === parseInt(match[1])) ?? null;
}

function decimal(value, decimals) {
  return String(Number(value.toFixed(decimals)));
}

function sourceSummary(probe, colourLabel) {
  const parts = [`${probe.width}x${probe.height}`];
  const fps = framesPerSecond(probe.fps);
  if (fps) parts.push(`${decimal(fps, FRAME_RATE_DECIMALS)} fps`);
  parts.push(colourLabel);
  if (probe.bitRate) parts.push(`${decimal(probe.bitRate / BITS_PER_MEGABIT, BIT_RATE_DECIMALS)} Mbit/s`);
  return `From the source: ${parts.join(", ")}. Edit any field to override.`;
}

// bandwidth is the encode target, not the source's bit rate
export function detectedValues(probe, framerateOptions) {
  const colour = detectedColour(probe);
  return {
    framerate: detectedFramerate(probe.fps, framerateOptions),
    sourceColourspace: colour.sourceColourspace,
    hint: sourceSummary(probe, colour.label),
  };
}

export function probeMayFill({ detected, profileDriven, edited, atDefault }) {
  return !profileDriven && !edited && (detected || atDefault);
}

export function autoResolutionLabel([width, height]) {
  return `Auto (${width}×${height})`;
}
