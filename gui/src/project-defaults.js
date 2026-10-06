import { FORM_CONTROLS } from "./project-form.js";

const PREFERRED_FORM_KEYS = new Set(["standard", "resolution", "framerate", "bandwidth"]);

// [form key, control id] of the Properties controls a new project takes from Settings
export const PREFERRED_PROJECT_CONTROLS = FORM_CONTROLS
  .filter(([key]) => PREFERRED_FORM_KEYS.has(key))
  .map(([key, id]) => [key, id]);

function matchingOption(options, value) {
  if (!options) return value;
  return options.find((option) => option.toLowerCase() === value.toLowerCase());
}

// Settings spells SMPTE and Interop in capitals, Properties in lowercase
export function projectDefaultsFromPreferences(preferences, optionValuesByKey) {
  const defaults = {};
  for (const key of PREFERRED_FORM_KEYS) {
    const value = matchingOption(optionValuesByKey[key], String(preferences[key]));
    if (value !== undefined) defaults[key] = value;
  }
  return defaults;
}
