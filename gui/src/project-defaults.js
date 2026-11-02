import { FORM_CONTROLS } from "./project-form.js";

const PREFERRED_FORM_KEYS = new Set(["isdcfNaming", "standard", "resolution", "framerate", "bandwidth"]);

// [form key, control id, control property] of the Properties controls a new project takes from Settings
export const PREFERRED_PROJECT_CONTROLS = FORM_CONTROLS.filter(([key]) => PREFERRED_FORM_KEYS.has(key));

function matchingOption(options, value) {
  if (!options) return value;
  return options.find((option) => option.toLowerCase() === value.toLowerCase());
}

function preferredValue(saved, options) {
  if (typeof saved === "boolean") return saved;
  return matchingOption(options, String(saved));
}

// Settings spells SMPTE and Interop in capitals, Properties in lowercase
export function projectDefaultsFromPreferences(preferences, optionValuesByKey) {
  const defaults = {};
  for (const key of PREFERRED_FORM_KEYS) {
    const value = preferredValue(preferences[key], optionValuesByKey[key]);
    if (value !== undefined) defaults[key] = value;
  }
  return defaults;
}
