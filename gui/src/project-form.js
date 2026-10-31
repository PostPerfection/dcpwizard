import { ratingRowsFrom } from "./rating-rows.js";
import { DEFAULT_TIMELINE_FPS, emptyTimeline, parseFrameRate, timelineWithKnownAssets } from "../../extern/guikit/src/composition-timeline.js";
import { isChannelSet } from "./channel-set.js";
import { rowAutoRoute } from "../../extern/guikit/src/audio-map-matrix.js";

const VALUE = "value";
const CHECKED = "checked";

export const PROJECT_FILE_VERSION = 4;

function withoutChannelDirectory(form, upgradeNotes) {
  const { audioChannelDir, ...rest } = form;
  if (audioChannelDir) upgradeNotes.push(`Channel WAV directory ${audioChannelDir} is no longer read: drop the WAVs on Assets instead`);
  return rest;
}

function migratedTimelineRate(form, assetsById, reels) {
  const firstPicture = assetsById.get(reels.find((reel) => reel.picture)?.picture);
  return parseFrameRate(firstPicture?.fps) ?? parseFrameRate(form.framerate) ?? DEFAULT_TIMELINE_FPS;
}

// frames stay 0 until the restore probes the source
function migratedClip(picture, sound, subtitle) {
  return { picture, frames: 0, inFrame: 0, outFrame: 0, sound, subtitle };
}

function timelineFromReels(composition, form, assetsById, upgradeNotes, reelStartingClipIndices) {
  const timeline = emptyTimeline(migratedTimelineRate(form, assetsById, composition.reels));
  composition.reels.forEach((reel, index) => {
    const reelName = `${composition.name} reel ${index + 1}`;
    const picture = reel.picture ?? null;
    const sound = reel.sound ?? null;
    const subtitle = reel.subtitle ?? null;
    const soundIsChannelSet = isChannelSet(assetsById.get(sound));
    if (index === 0 && soundIsChannelSet) timeline.sound = sound;
    if (picture === null) {
      if (index === 0) {
        timeline.sound = sound;
        timeline.subtitle = subtitle;
        return;
      }
      if (sound !== null) upgradeNotes.push(`${reelName} sound stays off the timeline: the reel has no picture`);
      if (subtitle !== null) upgradeNotes.push(`${reelName} subtitle stays off the timeline: the reel has no picture`);
      return;
    }
    const clipSound = sound === null || soundIsChannelSet ? { kind: "embedded", asset: null } : { kind: "file", asset: sound };
    timeline.clips.push(migratedClip(picture, clipSound, subtitle));
    if (index === 0) return;
    reelStartingClipIndices.push(timeline.clips.length - 1);
    upgradeNotes.push(`${reelName} is clip ${timeline.clips.length} of the timeline`);
    if (soundIsChannelSet) {
      upgradeNotes.push(`${reelName} sound stays off the timeline: a channel set only sounds over the whole timeline`);
    }
  });
  return timeline;
}

// a version 2 asset holds no frame count to place a reel start with
function reelsIntoTimeline(form, upgradeNotes) {
  if (!form.project) return form;
  const assetsById = new Map(form.project.assets.map((asset) => [asset.id, asset]));
  const compositions = form.project.compositions.map(({ reels, ...composition }) => {
    const reelStartingClipIndices = [];
    const timeline = timelineFromReels({ ...composition, reels }, form, assetsById, upgradeNotes, reelStartingClipIndices);
    return { ...composition, timeline, reelStartingClipIndices };
  });
  return { ...form, project: { ...form.project, compositions } };
}

// an unprobed clip is 0 frames long until withProbedFrames moves the clips after it along
function clipsEndToEnd(timeline) {
  let start = 0;
  const clips = timeline.clips.map((clip) => {
    const placed = { ...clip, start, layer: 0, opacity: 1 };
    start += clip.outFrame - clip.inFrame;
    return placed;
  });
  return { ...timeline, clips };
}

// version 3 clips play one after another, so each starts where the one before it ends
function clipsPlaced(form) {
  if (!form.project) return form;
  const compositions = form.project.compositions.map((composition) => ({ ...composition, timeline: clipsEndToEnd(composition.timeline) }));
  return { ...form, project: { ...form.project, compositions } };
}

// a rename or a meaning change gets a step and a version bump, a new field with a default does not
export const PROJECT_FILE_MIGRATIONS = { 2: withoutChannelDirectory, 3: reelsIntoTimeline, 4: clipsPlaced };

// [form key, control id, control property], keyed like the submit_job payload
export const FORM_CONTROLS = [
  ["title", "prop-title", VALUE],
  ["outputDir", "prop-output", VALUE],
  ["validate", "prop-validate", CHECKED],
  ["standard", "prop-standard", VALUE],
  ["resolution", "prop-resolution", VALUE],
  ["framerate", "prop-framerate", VALUE],
  ["bandwidth", "prop-bandwidth", VALUE],
  ["qualityPsnr", "prop-quality-psnr", VALUE],
  ["contentKind", "prop-content-kind", VALUE],
  ["encrypt", "prop-encrypt", CHECKED],
  ["keyOut", "prop-key-out", VALUE],
  ["rightEye", "prop-right-eye", VALUE],
  ["atmos", "prop-atmos", VALUE],
  ["subtitleLanguage", "prop-subtitle-language", VALUE],
  ["subtitleFontSize", "prop-subtitle-font-size", VALUE],
  ["subtitleColour", "prop-subtitle-colour", VALUE],
  ["subtitleEffect", "prop-subtitle-effect", VALUE],
  ["subtitleEffectColour", "prop-subtitle-effect-colour", VALUE],
  ["subtitleFadeUp", "prop-subtitle-fade-up", VALUE],
  ["subtitleFadeDown", "prop-subtitle-fade-down", VALUE],
  ["subtitleHalign", "prop-subtitle-halign", VALUE],
  ["subtitleValign", "prop-subtitle-valign", VALUE],
  ["subtitleVposition", "prop-subtitle-vposition", VALUE],
  ["subtitleZposition", "prop-subtitle-zposition", VALUE],
  ["subtitleRtl", "prop-subtitle-rtl", VALUE],
  ["subtitleWrap", "prop-subtitle-wrap", VALUE],
  ["subtitleFont", "prop-subtitle-font", VALUE],
  ["subtitleNoSubset", "prop-subtitle-no-subset", CHECKED],
  ["burnSubtitle", "prop-burn-subtitle", VALUE],
  ["subtitlesBurntIn", "prop-subtitles-burnt-in", CHECKED],
  ["burnSubtitleFont", "prop-burn-subtitle-font", VALUE],
  ["burnFontSize", "prop-burn-font-size", VALUE],
  ["burnColour", "prop-burn-colour", VALUE],
  ["burnEffect", "prop-burn-effect", VALUE],
  ["burnEffectColour", "prop-burn-effect-colour", VALUE],
  ["burnOutlineWidth", "prop-burn-outline-width", VALUE],
  ["burnLineHeight", "prop-burn-line-height", VALUE],
  ["burnMargin", "prop-burn-margin", VALUE],
  ["burnFadeUp", "prop-burn-fade-up", VALUE],
  ["burnFadeDown", "prop-burn-fade-down", VALUE],
  ["ccap", "prop-ccap", VALUE],
  ["ccapLanguage", "prop-ccap-language", VALUE],
  ["loudnessTarget", "prop-loudness", VALUE],
  ["truePeakCeiling", "prop-true-peak", VALUE],
  ["audioGainDb", "prop-audio-gain", VALUE],
  ["audioFadeInSeconds", "prop-audio-fade-in", VALUE],
  ["audioFadeOutSeconds", "prop-audio-fade-out", VALUE],
  ["audioInputOrder", "prop-audio-input-order", VALUE],
  ["audioChannels", "prop-audio-channels", VALUE],
  ["signLanguageVideo", "prop-sign-language-video", VALUE],
  ["signLanguageTag", "prop-sign-language-tag", VALUE],
  ["padHead", "prop-pad-head", VALUE],
  ["padTail", "prop-pad-tail", VALUE],
  ["padColor", "prop-pad-color", VALUE],
  ["audioDelayMs", "prop-audio-delay", VALUE],
  ["trimStart", "prop-trim-start", VALUE],
  ["trimEnd", "prop-trim-end", VALUE],
  ["stillLength", "prop-still-length", VALUE],
  ["videoFadeInSeconds", "prop-video-fade-in", VALUE],
  ["videoFadeOutSeconds", "prop-video-fade-out", VALUE],
  ["sourceColourspace", "prop-source-colourspace", VALUE],
  ["cropLeft", "prop-crop-left", VALUE],
  ["cropRight", "prop-crop-right", VALUE],
  ["cropTop", "prop-crop-top", VALUE],
  ["cropBottom", "prop-crop-bottom", VALUE],
  ["fillCrop", "prop-fill-crop", CHECKED],
  ["deinterlace", "prop-deinterlace", CHECKED],
  ["denoise", "prop-denoise", CHECKED],
  ["rotate", "prop-rotate", VALUE],
  ["flip", "prop-flip", VALUE],
  ["pictureScalePercent", "prop-picture-scale", VALUE],
  ["pictureOffsetX", "prop-picture-offset-x", VALUE],
  ["pictureOffsetY", "prop-picture-offset-y", VALUE],
  ["upmix", "prop-upmix", VALUE],
  ["reelLengthMinutes", "prop-reel-length", VALUE],
  ["splitAt", "prop-split-at", VALUE],
  ["splitChapters", "prop-split-chapters", CHECKED],
  ["versions", "prop-versions", VALUE],
  ["hdrDci", "prop-hdr-dci", CHECKED],
  ["hdrSource", "prop-hdr-source", VALUE],
  ["hdrPeakNits", "prop-hdr-peak-nits", VALUE],
  ["hdrToDciLut", "prop-hdr-lut", VALUE],
  ["hdrAlreadyPq", "prop-hdr-already-pq", CHECKED],
  ["allowGenericHdrTonemap", "prop-hdr-generic-tonemap", CHECKED],
  ["audioLanguage", "prop-audio-language", VALUE],
  ["studio", "prop-studio", VALUE],
  ["territoryType", "prop-territory-type", VALUE],
  ["contentVersions", "prop-content-versions", VALUE],
  ["tempVersion", "prop-temp-version", CHECKED],
  ["preRelease", "prop-pre-release", CHECKED],
  ["redBand", "prop-red-band", CHECKED],
  ["twoDVersionOfThreeD", "prop-two-d-version-of-three-d", CHECKED],
  ["versionFile", "prop-version-file", CHECKED],
  ["versionNumber", "prop-version-number", VALUE],
  ["chain", "prop-chain", VALUE],
  ["distributor", "prop-distributor", VALUE],
  ["facilityName", "prop-facility-name", VALUE],
  ["luminance", "prop-luminance", VALUE],
  ["luminanceUnits", "prop-luminance-units", VALUE],
];

// the key output file is written by the build, so it need not exist yet
export const OUTPUT_FIELDS = ["keyOut"];

// form keys that belong to one title, which a saved preset leaves out
export const PROJECT_ONLY_FORM_KEYS = [
  "title",
  "outputDir",
  "keyOut",
  "rightEye",
  "atmos",
  "ccap",
  "signLanguageVideo",
  "burnSubtitle",
  "subtitlesBurntIn",
  "versions",
  "splitAt",
  "trimStart",
  "trimEnd",
  "stillLength",
  "versionNumber",
  "contentVersions",
  "tempVersion",
  "preRelease",
  "redBand",
  "twoDVersionOfThreeD",
  "versionFile",
];

// free text that can start with a slash, matched at any depth
export const TEXT_FIELDS = [
  "title",
  "name",
  "meta",
  "label",
  "position",
  "agency",
  "audioMap",
  "headItems",
  "tailItems",
  "audioLanguage",
  "studio",
  "contentVersions",
  "trimStart",
  "trimEnd",
  "stillLength",
  "splitAt",
  "padHead",
  "padTail",
  "padColor",
  "loudnessTarget",
  "subtitleLanguage",
  "subtitleColour",
  "subtitleEffectColour",
  "burnColour",
  "burnEffectColour",
  "ccapLanguage",
  "signLanguageTag",
  "chain",
  "distributor",
  "facilityName",
];

export function serializeForm({ elementById, project, markerRows, ratings, joinedItems, audioMap }) {
  const form = {};
  for (const [key, id, property] of FORM_CONTROLS) form[key] = elementById(id)[property];
  return {
    ...form,
    audioMap,
    project: {
      title: project.title,
      assets: project.assets.map((asset) => ({ ...asset })),
      compositions: project.compositions.map(({ reelStartingClipIndices, ...composition }) => ({
        ...composition,
        timeline: structuredClone(composition.timeline),
      })),
      activeComposition: project.activeComposition,
    },
    markerRows: markerRows.map(({ label, position }) => ({ label, position })),
    ratings: ratings.map(({ agency, label }) => ({ agency, label })),
    headItems: [...joinedItems.head],
    tailItems: [...joinedItems.tail],
  };
}

export function offersOption(select, value) {
  return [...select.options].some((option) => option.value === value);
}

// rows get fresh ids from 1, so the caller restarts its id counters after them
export function restoreFormState(savedForm, defaults, { elementById, project, markerRows, ratings, joinedItems }) {
  const form = { ...defaults, ...savedForm, project: { ...defaults.project, ...savedForm.project } };
  const notRestored = [];
  for (const [key, id, property] of FORM_CONTROLS) {
    const element = elementById(id);
    const offered = !element.options || offersOption(element, form[key]);
    if (!offered) notRestored.push(`${element.labels[0].textContent.trim()} option ${form[key]}`);
    element[property] = offered ? form[key] : defaults[key];
  }

  const assets = form.project.assets.map((asset) => ({ ...asset }));
  const assetsById = new Map(assets.map((asset) => [asset.id, asset]));
  project.title = form.project.title;
  project.assets = assets;
  project.compositions = form.project.compositions.map((composition) => ({
    ...composition,
    timeline: timelineWithKnownAssets(composition, assetsById, notRestored),
  }));
  project.activeComposition = form.project.activeComposition;

  markerRows.splice(0, markerRows.length, ...form.markerRows.map(({ label, position }, index) => ({ id: index + 1, label, position })));
  ratings.splice(0, ratings.length, ...ratingRowsFrom(form.ratings));
  joinedItems.head = [...form.headItems];
  joinedItems.tail = [...form.tailItems];
  return { form, notRestored };
}

// a bare IN:LANE pair is routed at 0 dB
export function audioMapCells(spec) {
  if (!spec) return [];
  return spec.split(",").map((entry) => {
    const [pair, gain = "0"] = entry.split("@");
    const [input, lane] = pair.split(":");
    return { input, lane, gain };
  });
}

// row n is input n, and an untyped row keeps its automatic lane once anything is typed
export function audioMapSpecFrom(rows) {
  if (!rows.some((row) => row.typed.length)) return null;
  const entries = rows.flatMap((row, index) => {
    const input = index + 1;
    const autoRoute = rowAutoRoute(row);
    if (autoRoute) return [`${input}:${autoRoute}`];
    return row.typed.map(({ lane, gain }) => (parseFloat(gain) === 0 ? `${input}:${lane}` : `${input}:${lane}@${gain}`));
  });
  return entries.join(",");
}
