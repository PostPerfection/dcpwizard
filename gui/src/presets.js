import { invoke } from "@tauri-apps/api/core";
import { open, save, confirm, message } from "@tauri-apps/plugin-dialog";
import { askForText } from "../../extern/guikit/src/text-dialog.js";
import { FORM_CONTROLS, PROJECT_ONLY_FORM_KEYS, offersOption } from "./project-form.js";

const PROFILE_DRIVEN_CLASS = "profile-driven";
const SAVED_PRESETS_GROUP_CLASS = "saved-presets";
const SAVED_PRESETS_GROUP_LABEL = "Saved";
const PRESETS_FILE_FILTERS = [{ name: "Presets", extensions: ["json"] }];
const EXPORTED_PRESETS_FILE_NAME = "dcpwizard-presets.json";

const PRESET_CONTROLS = FORM_CONTROLS.filter(([key]) => !PROJECT_ONLY_FORM_KEYS.includes(key));

export function presetSnapshot(name, elementById, audioMap) {
  const form = Object.fromEntries(PRESET_CONTROLS.map(([key, id, property]) => [key, elementById(id)[property]]));
  return { name, form, audioMap };
}

// the controls it set, and the select values the panel no longer offers
export function applyPresetForm(preset, elementById) {
  const applied = [];
  const notApplied = [];
  for (const [key, id, property] of PRESET_CONTROLS) {
    if (!(key in preset.form)) continue;
    const element = elementById(id);
    const value = preset.form[key];
    if (element.options && !offersOption(element, value)) {
      notApplied.push(`${element.labels[0].textContent.trim()} option ${value}`);
      continue;
    }
    element[property] = value;
    applied.push(element);
  }
  return { applied, notApplied };
}

function counted(count, noun) {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

export function presetHint(name, settingCount) {
  return `Set by ${name}: ${counted(settingCount, "setting")}. Edit any field to override.`;
}

export function appliedStatus(name, notApplied) {
  const status = `Preset ${name} applied`;
  return notApplied.length ? `${status}, not applied: ${notApplied.join(", ")}` : status;
}

export function importStatus({ imported, replaced }) {
  const status = `Imported ${counted(imported.length, "preset")}`;
  return replaced.length ? `${status}, replaced ${replaced.join(", ")}` : status;
}

export function clearProfileDriven() {
  for (const element of document.querySelectorAll(`.${PROFILE_DRIVEN_CLASS}`)) element.classList.remove(PROFILE_DRIVEN_CLASS);
}

let savedPresets = [];
let panel = null;

export function savedPresetNamed(name) {
  return savedPresets.find((preset) => preset.name === name) ?? null;
}

function elementById(id) {
  return document.getElementById(id);
}

function refreshDeleteButton() {
  panel.deleteButton.disabled = !savedPresetNamed(panel.select.value);
}

function renderSavedPresets() {
  const { select } = panel;
  const selected = select.value;
  select.querySelector(`optgroup.${SAVED_PRESETS_GROUP_CLASS}`)?.remove();
  if (savedPresets.length) {
    const group = document.createElement("optgroup");
    group.className = SAVED_PRESETS_GROUP_CLASS;
    group.label = SAVED_PRESETS_GROUP_LABEL;
    for (const preset of savedPresets) group.appendChild(new Option(preset.name, preset.name));
    select.appendChild(group);
  }
  select.value = offersOption(select, selected) ? selected : "";
  refreshDeleteButton();
}

export async function applySavedPreset(preset) {
  clearProfileDriven();
  const { applied, notApplied } = applyPresetForm(preset, elementById);
  for (const element of applied) {
    element.classList.remove("detected");
    element.classList.add(PROFILE_DRIVEN_CLASS);
  }
  const unroutedCells = await panel.fillAudioMap(preset.audioMap);
  if (unroutedCells) notApplied.push(`${unroutedCells} audio map routes`);
  const setsAudioMap = Boolean(preset.audioMap);
  panel.audioMapGrid.classList.toggle(PROFILE_DRIVEN_CLASS, setsAudioMap);
  panel.hint.textContent = presetHint(preset.name, applied.length + (setsAudioMap ? 1 : 0));
  panel.afterApply();
  panel.setStatus(appliedStatus(preset.name, notApplied));
}

async function failed(title, error) {
  await message(String(error), { title, kind: "error" });
}

async function saveAsPreset() {
  const name = await askForText({
    title: "Save as preset",
    label: "Preset name",
    value: savedPresetNamed(panel.select.value)?.name ?? "",
  });
  if (!name) return;
  const replacing = savedPresetNamed(name) !== null;
  if (replacing && !(await confirm(`Replace the saved preset ${name}?`, { title: "Save as preset", kind: "warning" }))) return;
  try {
    savedPresets = await invoke("save_preset", { preset: presetSnapshot(name, elementById, panel.audioMapSpec()) });
  } catch (error) {
    await failed("Save preset failed", error);
    return;
  }
  renderSavedPresets();
  panel.select.value = name;
  refreshDeleteButton();
  await applySavedPreset(savedPresetNamed(name));
  panel.setStatus(`${replacing ? "Replaced" : "Saved"} preset ${name}`);
}

async function deleteSelectedPreset() {
  const preset = savedPresetNamed(panel.select.value);
  if (!preset) return;
  if (!(await confirm(`Delete the saved preset ${preset.name}?`, { title: "Delete preset", kind: "warning" }))) return;
  try {
    savedPresets = await invoke("delete_preset", { name: preset.name });
  } catch (error) {
    await failed("Delete preset failed", error);
    return;
  }
  renderSavedPresets();
  panel.select.dispatchEvent(new Event("change"));
  panel.setStatus(`Deleted preset ${preset.name}`);
}

async function exportPresets() {
  const destination = await save({ defaultPath: EXPORTED_PRESETS_FILE_NAME, filters: PRESETS_FILE_FILTERS });
  if (!destination) return;
  try {
    await invoke("export_presets", { destination });
  } catch (error) {
    await failed("Export presets failed", error);
    return;
  }
  panel.setStatus(`Exported ${counted(savedPresets.length, "preset")} to ${destination}`);
}

async function importPresets() {
  const source = await open({ multiple: false, directory: false, filters: PRESETS_FILE_FILTERS });
  if (!source) return;
  let result;
  try {
    result = await invoke("import_presets", { source });
  } catch (error) {
    await failed("Import presets failed", error);
    return;
  }
  savedPresets = result.presets;
  renderSavedPresets();
  panel.setStatus(importStatus(result));
}

// call once the built-in profiles are in the select, so the saved group follows them
export async function initPresets({ select, hint, setStatus, fillAudioMap, audioMapSpec, afterApply }) {
  panel = {
    select,
    hint,
    setStatus,
    fillAudioMap,
    audioMapSpec,
    afterApply,
    audioMapGrid: elementById("prop-audio-map"),
    deleteButton: elementById("prop-preset-delete"),
  };
  elementById("prop-preset-save").addEventListener("click", saveAsPreset);
  panel.deleteButton.addEventListener("click", deleteSelectedPreset);
  elementById("prop-preset-export").addEventListener("click", exportPresets);
  elementById("prop-preset-import").addEventListener("click", importPresets);
  select.addEventListener("change", refreshDeleteButton);
  elementById("properties").addEventListener("input", (event) => {
    event.target.closest(`.${PROFILE_DRIVEN_CLASS}`)?.classList.remove(PROFILE_DRIVEN_CLASS);
  });
  try {
    savedPresets = await invoke("list_presets");
  } catch (error) {
    setStatus(`Could not read the saved presets: ${error}`);
  }
  renderSavedPresets();
}
