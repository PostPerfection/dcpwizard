const VALUE = "value";
const CHECKED = "checked";

export const PROJECT_FILE_VERSION = 2;

// what an upgrade dropped, reported by restoreFormState as not restored
const DROPPED_ON_UPGRADE = "droppedOnUpgrade";

function withoutChannelDirectory(form) {
  const { audioChannelDir, ...rest } = form;
  if (!audioChannelDir) return rest;
  const dropped = `Channel WAV directory ${audioChannelDir}: drop the WAVs on Assets instead`;
  return { ...rest, [DROPPED_ON_UPGRADE]: [...(rest[DROPPED_ON_UPGRADE] ?? []), dropped] };
}

// a rename or a meaning change gets a step and a version bump, a new field with a default does not
export const PROJECT_FILE_MIGRATIONS = { 2: withoutChannelDirectory };

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

const REEL_SLOTS = ["picture", "sound", "subtitle"];

function assetId(asset) {
  return asset ? asset.id : null;
}

export function serializeForm({ elementById, project, markerRows, ratings, joinedItems, audioMap }) {
  const form = {};
  for (const [key, id, property] of FORM_CONTROLS) form[key] = elementById(id)[property];
  return {
    ...form,
    audioMap,
    project: {
      title: project.title,
      assets: project.assets.map((asset) => ({ ...asset })),
      compositions: project.compositions.map((composition) => ({
        ...composition,
        reels: composition.reels.map((reel) => ({
          id: reel.id,
          picture: assetId(reel.picture),
          sound: assetId(reel.sound),
          subtitle: assetId(reel.subtitle),
        })),
      })),
      activeComposition: project.activeComposition,
    },
    markerRows: markerRows.map(({ label, position }) => ({ label, position })),
    ratings: ratings.map(({ agency, label }) => ({ agency, label })),
    headItems: [...joinedItems.head],
    tailItems: [...joinedItems.tail],
  };
}

function offersOption(select, value) {
  return [...select.options].some((option) => option.value === value);
}

// rows get fresh ids from 1, so the caller restarts its id counters after them
export function restoreFormState(savedForm, defaults, { elementById, project, markerRows, ratings, joinedItems }) {
  const form = { ...defaults, ...savedForm, project: { ...defaults.project, ...savedForm.project } };
  const notRestored = [...(savedForm[DROPPED_ON_UPGRADE] ?? [])];
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
    reels: composition.reels.map((reel, index) => {
      const restoredReel = { id: reel.id };
      for (const slot of REEL_SLOTS) {
        const savedId = reel[slot] ?? null;
        restoredReel[slot] = assetsById.get(savedId) || null;
        if (savedId !== null && !restoredReel[slot]) notRestored.push(`${composition.name} reel ${index + 1} ${slot}`);
      }
      return restoredReel;
    }),
  }));
  project.activeComposition = form.project.activeComposition;

  markerRows.splice(0, markerRows.length, ...form.markerRows.map(({ label, position }, index) => ({ id: index + 1, label, position })));
  ratings.splice(0, ratings.length, ...form.ratings.map(({ agency, label }, index) => ({ id: index + 1, agency, label })));
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
    if (!row.typed.length) return row.autoRoute ? [`${input}:${row.autoRoute}`] : [];
    return row.typed.map(({ lane, gain }) => (parseFloat(gain) === 0 ? `${input}:${lane}` : `${input}:${lane}@${gain}`));
  });
  return entries.join(",");
}
