import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Command } from "@tauri-apps/plugin-shell";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { open as _open, save, confirm as tauriConfirm, message as tauriMessage } from "@tauri-apps/plugin-dialog";
import { initPreview, previewDcp, previewFile, previewNeedsContentKeys, coverPreviewSurface, uncoverPreviewSurface, previewPlayPause, previewSeek, previewSeekAbsolute, previewFrameStepBack, previewFrameStepForward, PREVIEW_SEEK_SECONDS, isPreviewVisible, setPreviewCrop, setPreviewSubtitleFile, setPreviewCaptionFile, watchPreviewShown } from "../../extern/guikit/src/preview.js";
import { previewTarget, previewButtonEnabled, PREVIEW_KIND_SOURCE } from "./preview-target.js";
import { progressDisplay } from "./progress-format.js";
import { markerSpecs } from "./marker-specs.js";
import { compositionFrameAt, durationFrames, markerFromPlayerUnavailable, markerTimecode } from "./marker-from-player.js";
import { initPlaylist, addToPlaylist } from "../../extern/guikit/src/playlist.js";
import { initJobsPanel, refreshJobs, startJobsPolling, stopJobsPolling } from "../../extern/guikit/src/jobs.js";
import * as buildsInFlight from "../../extern/guikit/src/builds-in-flight.js";
import { initTimeline, loadTimelineFromCpl } from "./timeline.js";
import { initShortcuts, getBinding } from "../../extern/guikit/src/shortcuts.js";
import { askForText } from "../../extern/guikit/src/text-dialog.js";
import { loadComponentVersions } from "../../extern/guikit/src/component-versions.js";
import { documentsOrHomeDir } from "../../extern/guikit/src/folders.js";
import { initProjects, PROJECT_FILE_SHORTCUTS, saveProjectBesidePackage, projectPathBeside, moveProjectFile, addRecentProject, getRecentProjects, renderRecentProjects, setWindowTitleStatus } from "../../extern/guikit/src/project.js";
import { serializeForm, restoreFormState, audioMapCells, OUTPUT_FIELDS, TEXT_FIELDS, PROJECT_FILE_VERSION, PROJECT_FILE_MIGRATIONS } from "./project-form.js";
import { initAssetStripResize } from "../../extern/guikit/src/asset-strip-resize.js";
import { dropIntoJoin, joinedPayload, libraryPayload } from "./library-joins.js";
import { exportRequestFrom, exportProgressText, exportProgressPercent, withMovieExtension, movieExtensions, isMovieFormat, takesCrf } from "./export-form.js";
import { contentKeysFrom } from "./content-keys-form.js";
import { prefilledRecipientKey } from "./recipient-identity.js";
import {
  chainPaths,
  describeSigner,
  kdmRequest,
  recipientCertificateCommand,
  recipientCreationRefusals,
  recipientPaths,
  rewrapRequest,
  signingChainCommand,
  signingChainFrom,
} from "./kdm-form.js";
initAssetStripResize("dcpwizard");

// === Browse wrapper (remembers last directory) ===
const LAST_BROWSE_DIR_KEY = "dcpwizard-last-browse-dir";
let lastBrowseDir = localStorage.getItem(LAST_BROWSE_DIR_KEY);
async function open(opts = {}) {
  const result = await _open({ ...opts, defaultPath: opts.defaultPath || lastBrowseDir || undefined });
  if (result) {
    lastBrowseDir = opts.directory ? result : result.replace(/[/\\][^/\\]*$/, '');
    localStorage.setItem(LAST_BROWSE_DIR_KEY, lastBrowseDir);
  }
  return result;
}

// === Sidebar navigation ===
document.querySelectorAll(".sidebar-btn[data-view]").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".sidebar-btn").forEach((b) => b.classList.remove("active"));
    document.querySelectorAll(".view").forEach((v) => v.classList.remove("active"));
    btn.classList.add("active");
    const view = document.getElementById(`view-${btn.dataset.view}`);
    if (view) view.classList.add("active");

    // Auto-refresh jobs when switching to jobs view
    if (btn.dataset.view === "jobs") {
      refreshJobs();
      startJobsPolling();
    } else {
      stopJobsPolling();
    }
  });
});

// === Theme toggle ===
document.getElementById("set-gpu-license-show")?.addEventListener("click", (event) => {
  const license = document.getElementById("set-gpu-license");
  const hidden = license.type === "password";
  license.type = hidden ? "text" : "password";
  event.currentTarget.textContent = hidden ? "Hide" : "Show";
});

document.getElementById("theme-toggle")?.addEventListener("click", () => {
  document.body.classList.toggle("light");
  const btn = document.getElementById("theme-toggle");
  btn.textContent = document.body.classList.contains("light") ? "☀️" : "🌙";
});

// === Keyboard shortcuts ===
function switchView(viewName) {
  document.querySelectorAll(".sidebar-btn").forEach((b) => b.classList.remove("active"));
  document.querySelectorAll(".view").forEach((v) => v.classList.remove("active"));
  const btn = document.querySelector(`.sidebar-btn[data-view="${viewName}"]`);
  if (btn) btn.classList.add("active");
  const view = document.getElementById(`view-${viewName}`);
  if (view) view.classList.add("active");
  if (viewName === "jobs") { refreshJobs(); startJobsPolling(); } else { stopJobsPolling(); }
}

const SHORTCUTS_KEY = "dcpwizard-shortcuts";

const PROJECT_BUTTON_SHORTCUTS = [
  ...PROJECT_FILE_SHORTCUTS,
  { id: "open-dcp", label: "Open DCP", binding: "Ctrl+Shift+O", buttonId: "btn-open-dcp" },
  { id: "build", label: "Build DCP", binding: "Ctrl+B", buttonId: "btn-build" },
  { id: "preview", label: "Preview", binding: "Ctrl+P", buttonId: "btn-preview" },
  { id: "import-video", label: "Import video", binding: "Ctrl+I", buttonId: "import-video" },
];
const THEME_BUTTON_SHORTCUT = { id: "toggle-theme", label: "Toggle light / dark theme", binding: "Ctrl+Shift+T", buttonId: "theme-toggle" };
const BUTTON_SHORTCUTS = [...PROJECT_BUTTON_SHORTCUTS, THEME_BUTTON_SHORTCUT];

function clickAction({ id, label, binding, buttonId }, category) {
  return { id, label, category, binding, handler: () => document.getElementById(buttonId)?.click() };
}

function viewAction(view, label, binding) {
  return { id: `view-${view}`, label, category: "Views", binding, handler: () => switchView(view) };
}

function previewAction(id, label, binding, handler) {
  return { id, label, category: "Preview", binding, when: isPreviewVisible, handler };
}

function refreshButtonTooltips() {
  for (const { id, label, buttonId } of BUTTON_SHORTCUTS) {
    const button = document.getElementById(buttonId);
    if (!button) continue;
    const binding = getBinding(id);
    button.title = binding ? `${label} (${binding})` : label;
  }
}

initShortcuts({
  storageKey: SHORTCUTS_KEY,
  onChange: refreshButtonTooltips,
  actions: [
    ...PROJECT_BUTTON_SHORTCUTS.map((shortcut) => clickAction(shortcut, "Project")),
    viewAction("project", "Project", "Ctrl+1"),
    viewAction("reels", "Reels & Timeline", "Ctrl+2"),
    viewAction("verify", "Verify", "Ctrl+3"),
    viewAction("security", "Encryption & KDM", "Ctrl+4"),
    viewAction("tools", "Tools", "Ctrl+5"),
    viewAction("jobs", "Jobs", "Ctrl+6"),
    viewAction("settings", "Settings", "Ctrl+7"),
    previewAction("preview-play-pause", "Play / pause", "Space", previewPlayPause),
    previewAction("preview-back", `Back ${PREVIEW_SEEK_SECONDS} seconds`, "ArrowLeft", () => previewSeek(-PREVIEW_SEEK_SECONDS)),
    previewAction("preview-forward", `Forward ${PREVIEW_SEEK_SECONDS} seconds`, "ArrowRight", () => previewSeek(PREVIEW_SEEK_SECONDS)),
    previewAction("preview-frame-back", "Step back one frame", ",", previewFrameStepBack),
    previewAction("preview-frame-forward", "Step forward one frame", ".", previewFrameStepForward),
    previewAction("preview-start", "Go to start", "Home", () => previewSeekAbsolute(0)),
    clickAction(THEME_BUTTON_SHORTCUT, "Appearance"),
  ],
});
refreshButtonTooltips();

// === Preferences ===
const PREFS_KEY = "dcpwizard-preferences";

// clear of DCI's 250 on purpose: at 250 exactly, rate allocation overshoot is a
// peak bitrate failure, and validators warn from 230 up
const DEFAULT_BANDWIDTH_MBPS = 230;
const DEFAULT_FRAMERATE = 24;
const AUTOMATIC_ENCODE_THREADS = 0;

const PREF_DEFAULTS = {
  standard: "SMPTE", resolution: "2K", framerate: DEFAULT_FRAMERATE,
  encrypt: false, stereo3d: false, validate: true,
  creator: "", facility: "", bandwidth: DEFAULT_BANDWIDTH_MBPS, gpu: false,
  gpuLicense: "", gpuRegistrationUrl: "", encodeThreads: AUTOMATIC_ENCODE_THREADS,
  signingCert: "", signingKey: "", signingIntermediate: "", signingIntermediateKey: "", signingRoot: "",
  recipientCert: "", recipientKey: "", outputDir: "", isdcfNaming: false,
  channels: "5.1", showHintsBeforeBuild: true, detectPictureFindings: false,
};

let currentPreferences = { ...PREF_DEFAULTS };

function getPrefs() {
  return { ...currentPreferences };
}

async function savePrefs(prefs) {
  currentPreferences = { ...PREF_DEFAULTS, ...prefs };
  try {
    await invoke("save_preferences", { preferences: currentPreferences });
    return true;
  } catch (error) {
    setStatus(`Could not save settings: ${error}`);
    return false;
  }
}

async function initializePreferences() {
  try {
    const loaded = await invoke("load_preferences");
    let legacy = {};
    try {
      legacy = JSON.parse(localStorage.getItem(PREFS_KEY)) || {};
    } catch {
      localStorage.removeItem(PREFS_KEY);
    }
    if (Object.keys(legacy).length > 0) {
      const migrated = { ...PREF_DEFAULTS, ...loaded, ...legacy };
      migrated.bandwidth = Math.min(migrated.bandwidth, DEFAULT_BANDWIDTH_MBPS);
      migrated.gpu = migrated.gpu === true;
      delete migrated._version;
      delete migrated.naming;
      currentPreferences = migrated;
      if (await savePrefs(migrated)) localStorage.removeItem(PREFS_KEY);
    } else {
      currentPreferences = { ...PREF_DEFAULTS, ...loaded };
    }
  } catch (error) {
    setStatus(`Could not load settings: ${error}`);
  }

  loadSettings();
  const preferences = getPrefs();
  await applyGpuSetting(
    preferences.gpu,
    preferences.gpuLicense,
    preferences.gpuRegistrationUrl,
    preferences.encodeThreads,
  );
  await loadComponentVersions(invoke);
}

const CERTIFICATE_SETTING_FIELDS = [
  { field: "signingCert", input: "set-signing-cert", browse: "set-browse-signing-cert" },
  { field: "signingKey", input: "set-signing-key", browse: "set-browse-signing-key" },
  { field: "signingIntermediate", input: "set-signing-intermediate", browse: "set-browse-signing-intermediate" },
  { field: "signingIntermediateKey", input: "set-signing-intermediate-key", browse: "set-browse-signing-intermediate-key" },
  { field: "signingRoot", input: "set-signing-root", browse: "set-browse-signing-root" },
  { field: "recipientCert", input: "set-recipient-cert", browse: "set-browse-recipient-cert" },
  { field: "recipientKey", input: "set-recipient-key", browse: "set-browse-recipient-key" },
];

function certificateFieldsFromForm() {
  return Object.fromEntries(
    CERTIFICATE_SETTING_FIELDS.map(({ field, input }) => [field, document.getElementById(input)?.value || ""]),
  );
}

async function saveCertificateFields() {
  return savePrefs({ ...getPrefs(), ...certificateFieldsFromForm() });
}

for (const { input, browse } of CERTIFICATE_SETTING_FIELDS) {
  document.getElementById(browse)?.addEventListener("click", async () => {
    const file = await open({ directory: false });
    if (file) document.getElementById(input).value = file;
  });
}

document.getElementById("set-export-recipient-cert")?.addEventListener("click", async () => {
  const certificate = document.getElementById("set-recipient-cert").value;
  const destination = await save({
    defaultPath: certificate.split(/[/\\]/).pop(),
    filters: [{ name: "Certificate", extensions: ["pem"] }],
  });
  if (!destination) return;
  try {
    await invoke("export_recipient_certificate", { certificate, destination });
    setStatus(`Exported the recipient certificate to ${destination}`);
  } catch (error) {
    setStatus(`Could not export the recipient certificate: ${error}`);
  }
});

document.getElementById("set-import-dcpomatic")?.addEventListener("click", async () => {
  let config;
  try {
    config = await invoke("find_dcpomatic_config");
  } catch (error) {
    setStatus(`${error}, choose the DCP-o-matic config.xml`);
    config = await open({ directory: false, filters: [{ name: "DCP-o-matic config", extensions: ["xml"] }] });
  }
  if (!config) return;
  try {
    const { preferences, thumbprint } = await invoke("import_dcpomatic_identity", { config });
    currentPreferences = { ...PREF_DEFAULTS, ...preferences };
    document.getElementById("set-recipient-cert").value = preferences.recipientCert;
    document.getElementById("set-recipient-key").value = preferences.recipientKey;
    setStatus(`Imported the DCP-o-matic recipient identity, certificate thumbprint ${thumbprint}`);
  } catch (error) {
    setStatus(`Could not import the DCP-o-matic recipient identity: ${error}`);
  }
});

async function runCertificateCommand(args) {
  const result = await Command.sidecar("dcpwizard", args).execute();
  if (result.code !== 0) {
    throw new Error((result.stderr || result.stdout || "the certificate command failed").trim());
  }
}

document.getElementById("set-create-signing-chain")?.addEventListener("click", async () => {
  const organization = await askForText({
    title: "Signing chain",
    label: "Organization name on the certificates",
    value: document.getElementById("set-creator")?.value || "",
  });
  if (!organization) return;
  const directory = await open({ directory: true });
  if (!directory) return;
  try {
    await runCertificateCommand(signingChainCommand(organization, directory));
  } catch (error) {
    setStatus(`Could not create the signing chain: ${error}`);
    return;
  }
  const paths = chainPaths(directory);
  for (const [field, path] of Object.entries(paths)) {
    const input = CERTIFICATE_SETTING_FIELDS.find((entry) => entry.field === field)?.input;
    const element = document.getElementById(input);
    if (element) element.value = path;
  }
  if (!await saveCertificateFields()) return;
  refreshSignerLine();
  setStatus(`Created a signing chain in ${directory}`);
});

document.getElementById("set-create-recipient")?.addEventListener("click", async () => {
  const current = certificateFieldsFromForm();
  const refusals = recipientCreationRefusals(current);
  if (refusals.length) {
    setStatus(refusals[0]);
    return;
  }
  const name = await askForText({
    title: "Recipient certificate",
    label: "Name for this machine",
    value: document.getElementById("set-creator")?.value || "DCP Wizard",
  });
  if (!name) return;
  const directory = await open({ directory: true });
  if (!directory) return;
  const command = recipientCertificateCommand({
    name,
    organization: document.getElementById("set-creator")?.value || name,
    intermediateCert: current.signingIntermediate,
    intermediateKey: current.signingIntermediateKey,
    directory,
  });
  try {
    await runCertificateCommand(command.args);
  } catch (error) {
    setStatus(`Could not create the recipient certificate: ${error}`);
    return;
  }
  const paths = recipientPaths(directory);
  document.getElementById("set-recipient-cert").value = paths.recipientCert;
  document.getElementById("set-recipient-key").value = paths.recipientKey;
  if (!await saveCertificateFields()) return;
  setStatus(`Created a recipient certificate in ${directory}`);
});

// Load prefs into settings form
function loadSettings() {
  const prefs = getPrefs();
  const map = {
    "set-standard": prefs.standard,
    "set-resolution": prefs.resolution,
    "set-framerate": prefs.framerate,
    "set-creator": prefs.creator,
    "set-facility": prefs.facility,
    "set-bandwidth": prefs.bandwidth,
    "set-output-dir": prefs.outputDir,
    "set-gpu-license": prefs.gpuLicense,
    "set-gpu-registration-url": prefs.gpuRegistrationUrl,
    "set-encode-threads": prefs.encodeThreads || "",
  };
  for (const { field, input } of CERTIFICATE_SETTING_FIELDS) map[input] = prefs[field];
  for (const [id, val] of Object.entries(map)) {
    const el = document.getElementById(id);
    if (el) el.value = val;
  }
  const naming = document.getElementById("set-isdcf-naming");
  if (naming) naming.checked = prefs.isdcfNaming;
  const showHints = document.getElementById("set-show-hints");
  if (showHints) showHints.checked = prefs.showHintsBeforeBuild;
  const detectPictureFindings = document.getElementById("set-detect-picture-findings");
  if (detectPictureFindings) detectPictureFindings.checked = prefs.detectPictureFindings;
  const gpu = document.getElementById("set-gpu");
  if (gpu) gpu.checked = prefs.gpu;
  refreshSignerLine();
}

// grok routes every compress and decompress in the process
async function applyGpuSetting(enabled, license, registrationUrl, encodeThreads) {
  try {
    const active = await invoke("set_gpu", {
      enabled,
      license: license || null,
      registrationUrl: registrationUrl || null,
      encodeThreads,
    });
    if (enabled && !active) throw new Error("Grok did not enable the GPU");
    return true;
  } catch (error) {
    setStatus(`GPU encoding unavailable: ${error}`);
    const gpu = document.getElementById("set-gpu");
    if (gpu) gpu.checked = false;
    await savePrefs({ ...getPrefs(), gpu: false });
    return false;
  }
}

// Advisory findings the pre-build check made. Returns true to build anyway.
function showHintsDialog(hints) {
  const dialog = document.getElementById("hints-dialog");
  const list = document.getElementById("hints-list");
  const silence = document.getElementById("hints-silence");
  if (!dialog || !list) return Promise.resolve(true);

  list.innerHTML = "";
  for (const hint of hints) {
    const item = document.createElement("li");
    item.textContent = hint;
    list.appendChild(item);
  }
  silence.checked = false;
  dialog.hidden = false;

  return new Promise((resolve) => {
    const close = (build) => {
      dialog.hidden = true;
      if (silence.checked) savePrefs({ ...getPrefs(), showHintsBeforeBuild: false });
      loadSettings();
      document.getElementById("hints-build").removeEventListener("click", onBuild);
      document.getElementById("hints-back").removeEventListener("click", onBack);
      resolve(build);
    };
    const onBuild = () => close(true);
    const onBack = () => close(false);
    document.getElementById("hints-build").addEventListener("click", onBuild);
    document.getElementById("hints-back").addEventListener("click", onBack);
  });
}

document.getElementById("settings-form")?.addEventListener("submit", async (e) => {
  e.preventDefault();
  const prefs = {
    ...getPrefs(),
    standard: document.getElementById("set-standard")?.value,
    resolution: document.getElementById("set-resolution")?.value,
    framerate: parseInt(document.getElementById("set-framerate")?.value) || DEFAULT_FRAMERATE,
    creator: document.getElementById("set-creator")?.value,
    facility: document.getElementById("set-facility")?.value,
    bandwidth: parseInt(document.getElementById("set-bandwidth")?.value) || DEFAULT_BANDWIDTH_MBPS,
    ...Object.fromEntries(
      CERTIFICATE_SETTING_FIELDS.map(({ field, input }) => [field, document.getElementById(input)?.value]),
    ),
    outputDir: document.getElementById("set-output-dir")?.value,
    isdcfNaming: document.getElementById("set-isdcf-naming")?.checked || false,
    showHintsBeforeBuild: !!document.getElementById("set-show-hints")?.checked,
    detectPictureFindings: !!document.getElementById("set-detect-picture-findings")?.checked,
    gpu: !!document.getElementById("set-gpu")?.checked,
    gpuLicense: document.getElementById("set-gpu-license")?.value.trim() || "",
    gpuRegistrationUrl: document.getElementById("set-gpu-registration-url")?.value.trim() || "",
    encodeThreads:
      parseInt(document.getElementById("set-encode-threads")?.value) || AUTOMATIC_ENCODE_THREADS,
  };
  if (!await applyGpuSetting(
    prefs.gpu,
    prefs.gpuLicense,
    prefs.gpuRegistrationUrl,
    prefs.encodeThreads,
  )) return;
  if (!await savePrefs(prefs)) return;
  refreshSignerLine();
  refreshIsdcfPreview();
  setStatus("Settings saved");
});

document.getElementById("set-reset")?.addEventListener("click", async () => {
  try {
    await invoke("reset_preferences");
    localStorage.removeItem(PREFS_KEY);
    location.reload();
  } catch (error) {
    setStatus(`Could not reset settings: ${error}`);
  }
});

initializePreferences();

// === Project State ===
const project = {
  title: "",
  assets: [],  // {id, type: 'video'|'audio'|'subtitle', path, name, meta}
  compositions: [
    { id: 1, name: "Main", contentKind: "feature", reels: [{ id: 1, picture: null, sound: null, subtitle: null }] }
  ],
  activeComposition: 0,  // index into compositions[]
};

// Convenience accessor for active composition reels
function getActiveReels() {
  return project.compositions[project.activeComposition]?.reels || [];
}
function setActiveReels(reels) {
  if (project.compositions[project.activeComposition]) {
    project.compositions[project.activeComposition].reels = reels;
  }
}

// Legacy alias for backward compat in this file
Object.defineProperty(project, 'reels', {
  get() { return getActiveReels(); },
  set(v) { setActiveReels(v); },
  configurable: true,
});

let nextAssetId = 1;

// === OS file drop (Tauri webview drag-drop) ===
// A webview's HTML drop event exposes no filesystem path (f.path is Electron
// only). Tauri's native drag-drop event carries absolute paths.
const dropOverlay = document.getElementById("drop-overlay");

getCurrentWebview().onDragDropEvent((event) => {
  const p = event.payload;
  if (p.type === "over" || p.type === "enter") {
    if (dropOverlay) dropOverlay.hidden = false;
  } else if (p.type === "drop") {
    if (dropOverlay) dropOverlay.hidden = true;
    for (const path of p.paths) {
      importAssetFromPath(path, guessType(path));
    }
  } else {
    // "leave" / "cancel"
    if (dropOverlay) dropOverlay.hidden = true;
  }
});

function guessType(name) {
  const ext = name.split('.').pop().toLowerCase();
  if (['mp4','mkv','mov','avi','mxf','webm','j2c','tiff','tif','dpx','exr'].includes(ext)) return 'video';
  if (['wav','aiff','flac','mp3','pcm'].includes(ext)) return 'audio';
  if (['xml','ttml','srt','vtt'].includes(ext)) return 'subtitle';
  return 'video';
}

// === Asset import ===
document.getElementById("import-video")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Video', extensions: ['mp4','mkv','mov','avi','mxf','webm'] },
      { name: 'Still image', extensions: ['png','jpg','jpeg','tif','tiff','bmp','dpx','exr'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) importAssetFromPath(path, 'video');
});

document.getElementById("import-audio")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Audio', extensions: ['wav','aiff','flac','mp3'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) importAssetFromPath(path, 'audio');
});

document.getElementById("import-subtitle")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Subtitle', extensions: ['xml','ttml','srt','vtt'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) importAssetFromPath(path, 'subtitle');
});

const REEL_SLOT_BY_ASSET_TYPE = { video: 'picture', audio: 'sound', subtitle: 'subtitle' };

function placedAssetIds() {
  return new Set(project.reels.flatMap(r => [r.picture, r.sound, r.subtitle]).filter(Boolean).map(a => a.id));
}

// the first reel takes the first asset of each type no reel holds yet
function fillFirstReelFromUnplacedAssets() {
  const reel = project.reels[0];
  const placed = placedAssetIds();
  for (const [type, slot] of Object.entries(REEL_SLOT_BY_ASSET_TYPE)) {
    if (reel[slot]) continue;
    reel[slot] = project.assets.find(a => a.type === type && !placed.has(a.id)) ?? null;
  }
}

function importAssetFromPath(path, type) {
  const name = path.split(/[/\\]/).pop();
  const asset = { id: nextAssetId++, type, path, name, meta: '' };
  project.assets.push(asset);

  const reel = project.reels[0];
  const slot = REEL_SLOT_BY_ASSET_TYPE[type];
  if (slot && !reel[slot]) reel[slot] = asset;

  renderAssets();
  renderReels();
  updateStatusStats();
  setStatus(`Imported: ${name}`);

  // Auto-detect video properties
  if (type === 'video') {
    probeVideo(path).then(info => {
      if (!info) return;
      asset.meta = `${info.width}×${info.height} ${info.fps}`;
      asset.width = info.width;
      asset.height = info.height;
      if (project.assets.filter(a => a.type === 'video').length === 1) {
        // Pre-fill resolution from first video
        const resEl = document.getElementById("prop-resolution");
        if (resEl && resEl.value === "auto") {
          // Keep auto — the backend will handle it
        }
        // Pre-fill framerate
        const fpsMatch = info.fps?.match(/^(\d+)\/1$/);
        if (fpsMatch) {
          const fpsEl = document.getElementById("prop-framerate");
          if (fpsEl) {
            const fps = parseInt(fpsMatch[1]);
            for (const opt of fpsEl.options) {
              if (parseInt(opt.value) === fps) { fpsEl.value = opt.value; break; }
            }
          }
        }
      }
      refreshIsdcfPreview();
      renderAssets();
    });
  }
}

function renderAssets() {
  const list = document.getElementById("asset-list");
  if (!list) return;

  if (project.assets.length === 0) {
    list.innerHTML = '<div class="asset-empty"><p>Drag & drop video/audio files here<br>or use the buttons above</p></div>';
    return;
  }

  const icons = { video: '🎬', audio: '🔊', subtitle: '📝' };
  list.innerHTML = project.assets.map(a => `
    <div class="asset-item" data-asset-id="${a.id}" draggable="true">
      <span class="asset-icon">${icons[a.type]}</span>
      <span class="asset-name" title="${a.path}">${a.name}</span>
      <span class="asset-meta">${a.meta || a.type}</span>
      <button class="asset-remove" data-remove-id="${a.id}" title="Remove from project">✕</button>
    </div>
  `).join('');

  // Make assets draggable to reel tracks
  list.querySelectorAll('.asset-item').forEach(el => {
    el.addEventListener('dragstart', (e) => {
      e.dataTransfer.setData('text/plain', el.dataset.assetId);
    });
    el.addEventListener('contextmenu', (e) => {
      showContextMenu(e, parseInt(el.dataset.assetId));
    });
    el.addEventListener('click', () => {
      const asset = project.assets.find(a => a.id === parseInt(el.dataset.assetId));
      if (asset) selectPreview("source", asset.path);
    });
  });
  list.querySelectorAll('.asset-remove').forEach(el => {
    el.addEventListener('click', (e) => { e.stopPropagation(); removeAsset(parseInt(el.dataset.removeId)); });
  });

  // Re-apply filter
  const q = document.getElementById("asset-filter")?.value?.toLowerCase() || "";
  if (q) {
    list.querySelectorAll('.asset-item').forEach(el => {
      const name = el.querySelector(".asset-name")?.textContent?.toLowerCase() || "";
      el.style.display = name.includes(q) ? "" : "none";
    });
  }

  applyPreviewSelection();
}

function renderReels() {
  const list = document.getElementById("reel-list");
  if (!list) return;

  list.innerHTML = project.reels.map((reel, i) => `
    <div class="reel" data-reel="${reel.id}">
      <div class="reel-header">
        <span class="reel-label">Reel ${i + 1}</span>
        <span class="reel-duration">${reel.picture ? '—' : '--:--:--'}</span>
      </div>
      <div class="reel-tracks">
        <div class="track track-picture" data-reel-id="${reel.id}" data-track="picture">
          <span class="track-label">Picture</span>
          <span class="track-info ${reel.picture ? 'has-content' : ''}">${reel.picture ? reel.picture.name : 'Drop video here'}</span>
        </div>
        <div class="track track-sound" data-reel-id="${reel.id}" data-track="sound">
          <span class="track-label">Sound</span>
          <span class="track-info ${reel.sound ? 'has-content' : ''}">${reel.sound ? reel.sound.name : 'Drop audio here'}</span>
        </div>
        <div class="track track-subtitle" data-reel-id="${reel.id}" data-track="subtitle">
          <span class="track-label">Subtitle</span>
          <span class="track-info ${reel.subtitle ? 'has-content' : ''}">${reel.subtitle ? reel.subtitle.name : 'Optional'}</span>
        </div>
      </div>
    </div>
  `).join('');

  // Drop targets on reel tracks
  list.querySelectorAll('.track').forEach(track => {
    track.addEventListener('dragover', (e) => {
      e.preventDefault();
      track.style.background = 'var(--surface-hover)';
    });
    track.addEventListener('dragleave', () => {
      track.style.background = '';
    });
    track.addEventListener('drop', (e) => {
      e.preventDefault();
      track.style.background = '';
      const assetId = parseInt(e.dataTransfer.getData('text/plain'));
      const asset = project.assets.find(a => a.id === assetId);
      if (!asset) return;
      const reelId = parseInt(track.dataset.reelId);
      const reel = project.reels.find(r => r.id === reelId);
      if (!reel) return;
      const trackType = track.dataset.track;
      reel[trackType] = asset;
      renderReels();
    });
  });

  audioMapDrawn = refreshAudioMapMatrix();
  refreshIsdcfPreview();
  refreshMarkerFromPlayerButtons();
}

// === Source picture ===

document.getElementById("prop-auto-crop")?.addEventListener("click", async () => {
  const video = project.reels[0]?.picture?.path;
  const plan = document.getElementById("prop-crop-plan");
  if (!video) { tauriMessage("Import a video asset first"); return; }
  // a threshold of 0 means pure black only, so it must not fall back to the default
  const threshold = parseFloat(document.getElementById("prop-auto-crop-threshold")?.value);
  try {
    const crop = await invoke("detect_source_crop", {
      videoPath: video,
      threshold: Number.isNaN(threshold) ? null : threshold,
      resolution: document.getElementById("prop-resolution")?.value || null,
    });
    for (const side of CROP_SIDES) {
      const field = document.getElementById(`prop-crop-${side}`);
      if (field) field.value = crop[side];
    }
    refreshPreviewCrop();
    refreshIsdcfPreview();
    if (plan) plan.textContent = crop.description;
  } catch (e) {
    if (plan) plan.textContent = "";
    tauriMessage(String(e), { title: "Auto-crop failed", kind: "error" });
  }
});

// === Audio channel mapping matrix ===

// the path the drawn matrix belongs to, so re-rendering the reels does not throw
// away gains the user has typed
let audioMapPath = null;
let audioMapDrawn = Promise.resolve();

async function refreshAudioMapMatrix() {
  const grid = document.getElementById("prop-audio-map");
  const hint = document.getElementById("prop-audio-map-hint");
  if (!grid) return;
  const audio = project.reels[0]?.sound?.path || null;
  if (audio === audioMapPath) return;
  audioMapPath = audio;
  grid.innerHTML = "";
  if (!audio) {
    if (hint) hint.textContent = "Import a sound asset to map its channels.";
    return;
  }
  let panel;
  try {
    panel = await invoke("probe_audio_map", { audioPath: audio });
  } catch (e) {
    if (hint) hint.textContent = String(e);
    return;
  }
  if (hint) hint.textContent = "Empty leaves a channel unrouted. Click a cell to route it at 0 dB.";
  const header = panel.lanes.map(lane => `<th>${lane}</th>`).join("");
  const rows = Array.from({ length: panel.channels }, (_, channel) => {
    const cells = panel.lanes.map(lane =>
      `<td><input type="text" inputmode="decimal" data-input="${channel + 1}" data-lane="${lane}" title="Channel ${channel + 1} to ${lane}, gain in dB"></td>`
    ).join("");
    return `<tr><th>${channel + 1}</th>${cells}</tr>`;
  }).join("");
  grid.innerHTML = `<table><thead><tr><th></th>${header}</tr></thead><tbody>${rows}</tbody></table>`;
  grid.querySelectorAll("input").forEach(cell => {
    cell.addEventListener("focus", () => {
      if (!cell.value.trim()) cell.value = "0";
    });
  });
}

// The grid as an IN:LANE@GAIN spec, or null when nothing is routed.
function audioMapSpec() {
  const cells = document.querySelectorAll("#prop-audio-map input");
  const entries = [];
  for (const cell of cells) {
    const gain = cell.value.trim();
    if (!gain) continue;
    const pair = `${cell.dataset.input}:${cell.dataset.lane}`;
    entries.push(parseFloat(gain) === 0 ? pair : `${pair}@${gain}`);
  }
  return entries.length ? entries.join(",") : null;
}

// Add reel button
document.getElementById("add-reel")?.addEventListener("click", () => {
  const maxId = project.reels.reduce((m, r) => Math.max(m, r.id), 0);
  project.reels.push({ id: maxId + 1, picture: null, sound: null, subtitle: null });
  renderReels();
});

// === Multi-CPL Management ===
let nextCplId = 2;

function renderCplTabs() {
  const container = document.getElementById("cpl-tabs");
  if (!container) return;
  container.innerHTML = "";
  project.compositions.forEach((cpl, idx) => {
    const tab = document.createElement("button");
    tab.className = "cpl-tab" + (idx === project.activeComposition ? " active" : "");
    tab.dataset.cpl = idx;
    tab.textContent = cpl.name;
    if (project.compositions.length > 1) {
      const rm = document.createElement("span");
      rm.className = "cpl-tab-remove";
      rm.textContent = "\u00d7";
      rm.addEventListener("click", (e) => {
        e.stopPropagation();
        removeCpl(idx);
      });
      tab.appendChild(rm);
    }
    tab.addEventListener("click", () => switchCpl(idx));
    container.appendChild(tab);
  });
}

function switchCpl(idx) {
  if (idx < 0 || idx >= project.compositions.length) return;
  project.activeComposition = idx;
  renderCplTabs();
  renderReels();
  // Update properties panel content kind
  const cpl = project.compositions[idx];
  const kindEl = document.getElementById("prop-content-kind");
  if (kindEl && cpl.contentKind) kindEl.value = cpl.contentKind;
}

function removeCpl(idx) {
  if (project.compositions.length <= 1) return;
  project.compositions.splice(idx, 1);
  if (project.activeComposition >= project.compositions.length) {
    project.activeComposition = project.compositions.length - 1;
  }
  renderCplTabs();
  renderReels();
}

document.getElementById("add-cpl")?.addEventListener("click", async () => {
  const name = await askForText({
    title: "New composition",
    label: "Composition name",
    value: `CPL ${nextCplId}`,
  });
  if (!name) return;
  project.compositions.push({
    id: nextCplId++,
    name: name,
    contentKind: "feature",
    reels: [{ id: 1, picture: null, sound: null, subtitle: null }],
  });
  switchCpl(project.compositions.length - 1);
});

// Sync content kind changes to active composition
document.getElementById("prop-content-kind")?.addEventListener("change", (e) => {
  const cpl = project.compositions[project.activeComposition];
  if (cpl) cpl.contentKind = e.target.value;
});

// Initial render
renderCplTabs();

// === Output directory ===
async function defaultOutputFolder() {
  return getPrefs().outputDir || await documentsOrHomeDir();
}

function setOutputFolder(folder) {
  document.getElementById("prop-output").value = folder;
  refreshDiskSpace();
}

document.getElementById("browse-output")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) setOutputFolder(dir);
});

// === Open existing DCP ===
async function openDcp(dir) {
  let cpls;
  try {
    cpls = await invoke('list_cpls', { dcpDir: dir });
  } catch (error) {
    setStatus(`Open failed: ${error}`);
    return;
  }

  const name = dir.split(/[/\\]/).pop();
  project.title = name;
  document.getElementById("prop-title").value = name;
  setStatus(`Opened: ${dir}`);
  openedPackage = dir;
  selectPreview("package", dir);

  // Load timeline from the first CPL found
  if (cpls && cpls.length > 0) {
    const cplPath = dir + '/' + cpls[0].file_path;
    loadTimelineFromCpl(cplPath);
  }
}

document.getElementById("btn-open-dcp")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) openDcp(dir);
});

// === Build DCP ===
let currentJobId = null;
let paused = false;

document.getElementById("prop-browse-key-out")?.addEventListener("click", async () => {
  const file = await save({
    defaultPath: "dcp.keys.json",
    filters: [{ name: "Keys", extensions: ["json"] }],
  });
  if (file) document.getElementById("prop-key-out").value = file;
});

document.getElementById("prop-browse-right-eye")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Video', extensions: ['mp4','mkv','mov','avi','mxf','webm'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-right-eye").value = path;
});

document.getElementById("prop-browse-atmos")?.addEventListener("click", async () => {
  const path = await open({ directory: false, multiple: false });
  if (path) document.getElementById("prop-atmos").value = path;
});

// === Delivery profiles ===
// A profile fills the panel controls when it is picked. Whatever is in the
// controls at build time wins, so editing a field afterwards overrides the
// profile, and the edited field drops out of the "set by" hint.
const PROFILE_DRIVEN_FIELDS = [
  ["standard", "prop-standard", "standard"],
  ["resolution", "prop-resolution", "resolution"],
  ["framerate", "prop-framerate", "frame rate"],
  ["bandwidth", "prop-bandwidth", "bandwidth"],
  ["content_kind", "prop-content-kind", "content kind"],
];

let profileSettings = [];

function updateProfileHint(profileName, drivenLabels) {
  const hint = document.getElementById("prop-profile-hint");
  if (!hint) return;
  hint.textContent = drivenLabels.length
    ? `Set by ${profileName}: ${drivenLabels.join(", ")}. Edit any field to override it.`
    : "";
}

function applyProfile(profileName) {
  const driven = [];
  for (const [, elementId] of PROFILE_DRIVEN_FIELDS) {
    document.getElementById(elementId)?.classList.remove("profile-driven");
  }
  const profile = profileSettings.find((p) => p.name === profileName);
  if (!profile) {
    updateProfileHint("", driven);
    return;
  }
  for (const [key, elementId, label] of PROFILE_DRIVEN_FIELDS) {
    const value = profile[key];
    const element = document.getElementById(elementId);
    if (value === null || value === undefined || !element) continue;
    element.value = value;
    element.classList.add("profile-driven");
    driven.push(label);
  }
  updateProfileHint(profileName, driven);
  setStatus(`Profile ${profileName}: ${profile.description}`);
}

(async () => {
  const select = document.getElementById("prop-profile");
  if (!select) return;
  try {
    profileSettings = await invoke("list_profiles");
  } catch (e) {
    console.warn("[main] Could not load delivery profiles:", e);
    return;
  }
  for (const profile of profileSettings) {
    const option = document.createElement("option");
    option.value = profile.name;
    option.textContent = `${profile.name} — ${profile.description}`;
    select.appendChild(option);
  }
  select.addEventListener("change", (e) => applyProfile(e.target.value));
  for (const [, elementId] of PROFILE_DRIVEN_FIELDS) {
    document.getElementById(elementId)?.addEventListener("input", (e) => {
      e.target.classList.remove("profile-driven");
    });
  }
})();

document.getElementById("prop-browse-versions")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Versions manifest', extensions: ['json'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-versions").value = path;
});

document.getElementById("prop-browse-audio-channel-dir")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) document.getElementById("prop-audio-channel-dir").value = dir;
});

document.getElementById("prop-browse-sign-language-video")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Video', extensions: ['mp4','mkv','mov','avi','mxf','webm'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-sign-language-video").value = path;
});

document.getElementById("prop-browse-hdr-lut")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: '3D LUT', extensions: ['cube','3dl','csp','dat','m3d'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-hdr-lut").value = path;
});

document.getElementById("prop-browse-burn-subtitle")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Subtitles', extensions: ['srt','ass','ssa','pac','mks','mkv','fcpxml','xml'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-burn-subtitle").value = path;
});

document.getElementById("prop-browse-burn-subtitle-font")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Fonts', extensions: ['ttf','otf','ttc'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-burn-subtitle-font").value = path;
});

document.getElementById("prop-browse-subtitle-font")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Fonts', extensions: ['ttf','otf'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-subtitle-font").value = path;
});

document.getElementById("prop-browse-ccap")?.addEventListener("click", async () => {
  const path = await open({
    directory: false, multiple: false,
    filters: [
      { name: 'Captions', extensions: ['srt','xml','ttml','vtt'] },
      { name: 'All', extensions: ['*'] }
    ]
  });
  if (path) document.getElementById("prop-ccap").value = path;
});

// === Markers ===

const markerLabelsRequest = invoke("marker_labels");
let markerNextId = 1;
const markerRows = [];

function markerRowElement(markerRow, labels) {
  const element = document.createElement("div");
  element.className = "field-row marker-row";
  element.dataset.id = markerRow.id;
  element.innerHTML = `
    <div class="prop-field">
      <label>Marker</label>
      <select class="marker-label"></select>
    </div>
    <div class="prop-field">
      <label>Position</label>
      <input type="text" class="marker-position" placeholder="frames or HH:MM:SS:FF">
    </div>
    <button class="btn-sm marker-from-player" type="button">Set from player</button>
    <button class="btn-sm marker-remove" type="button" title="Remove marker">✕</button>
  `;
  const select = element.querySelector(".marker-label");
  for (const label of labels) {
    const option = document.createElement("option");
    option.value = label;
    option.textContent = label;
    select.appendChild(option);
  }
  select.value = markerRow.label;
  element.querySelector(".marker-position").value = markerRow.position;
  applyMarkerFromPlayerState(element.querySelector(".marker-from-player"));
  return element;
}

function markerFromPlayerBlocker() {
  return markerFromPlayerUnavailable({ compositionPicturePath: project.reels[0]?.picture?.path, shownPath: previewShownPath });
}

function applyMarkerFromPlayerState(button, blocker = markerFromPlayerBlocker()) {
  button.disabled = blocker !== null;
  button.title = blocker ?? "Write the preview's current frame into this marker";
}

function refreshMarkerFromPlayerButtons() {
  const blocker = markerFromPlayerBlocker();
  document.querySelectorAll("#prop-markers .marker-from-player").forEach(button => applyMarkerFromPlayerState(button, blocker));
}

function fieldFrames(id, label, editRate) {
  try {
    return durationFrames(document.getElementById(id)?.value || "", editRate);
  } catch (e) {
    throw new Error(`${label}: ${e.message}`);
  }
}

async function compositionTimecodeAtPlayer() {
  const blocker = markerFromPlayerBlocker();
  if (blocker) throw new Error(blocker);
  const picturePath = project.reels[0].picture.path;
  const editRate = parseInt(document.getElementById("prop-framerate")?.value) || DEFAULT_FRAMERATE;
  const [positionSeconds, durationSeconds, source] = await Promise.all([
    invoke("preview_get_position"),
    invoke("preview_get_duration"),
    probeVideo(picturePath),
  ]);
  const frame = compositionFrameAt({
    positionSeconds,
    durationSeconds,
    sourceRate: source?.fps,
    editRate,
    trimStartFrames: fieldFrames("prop-trim-start", "Trim start", editRate),
    trimEndFrames: fieldFrames("prop-trim-end", "Trim end", editRate),
    padHeadFrames: fieldFrames("prop-pad-head", "Head padding", editRate),
  });
  return markerTimecode(frame, editRate);
}

async function setMarkerFromPlayer(element) {
  const markerRow = markerRows.find(r => r.id === parseInt(element.dataset.id));
  try {
    markerRow.position = await compositionTimecodeAtPlayer();
  } catch (e) {
    tauriMessage(String(e.message ?? e), { title: "Marker not set", kind: "warning" });
    return;
  }
  element.querySelector(".marker-position").value = markerRow.position;
  setStatus(`${markerRow.label} set to ${markerRow.position}`);
}

document.getElementById("prop-add-marker")?.addEventListener("click", async () => {
  const labels = await markerLabelsRequest;
  const markerRow = { id: markerNextId++, label: labels[0], position: "" };
  markerRows.push(markerRow);
  document.getElementById("prop-markers").appendChild(markerRowElement(markerRow, labels));
});

document.getElementById("prop-markers")?.addEventListener("input", (e) => {
  const element = e.target.closest(".marker-row");
  const markerRow = markerRows.find(r => r.id === parseInt(element?.dataset.id));
  if (!markerRow) return;
  if (e.target.classList.contains("marker-label")) markerRow.label = e.target.value;
  if (e.target.classList.contains("marker-position")) markerRow.position = e.target.value;
});

document.getElementById("prop-markers")?.addEventListener("click", (e) => {
  if (e.target.classList.contains("marker-from-player")) {
    setMarkerFromPlayer(e.target.closest(".marker-row"));
    return;
  }
  if (!e.target.classList.contains("marker-remove")) return;
  const element = e.target.closest(".marker-row");
  const index = markerRows.findIndex(r => r.id === parseInt(element.dataset.id));
  markerRows.splice(index, 1);
  element.remove();
});

// === ISDCF naming and metadata ===

let ratingNextId = 1;
const ratings = [];

function renderRatings() {
  const list = document.getElementById("prop-rating-list");
  if (!list) return;
  list.innerHTML = ratings.map(rating => `
    <div class="field-row rating-row" data-id="${rating.id}">
      <div class="prop-field">
        <label>Agency (URI)</label>
        <input type="text" class="rating-agency" value="${rating.agency}" placeholder="http://www.mpaa.org/2003-ratings">
      </div>
      <div class="prop-field">
        <label>Label</label>
        <input type="text" class="rating-label" value="${rating.label}" placeholder="PG-13">
      </div>
      <button class="btn-sm rating-remove" type="button" title="Remove rating">✕</button>
    </div>
  `).join("");
  refreshIsdcfPreview();
}

document.getElementById("prop-add-rating")?.addEventListener("click", () => {
  ratings.push({ id: ratingNextId++, agency: "", label: "" });
  renderRatings();
});

// Rows are re-rendered, so edits and removal go through delegation. An edit must
// not re-render or the field being typed into loses focus.
document.getElementById("prop-rating-list")?.addEventListener("input", (e) => {
  const row = e.target.closest(".rating-row");
  const rating = ratings.find(r => r.id === parseInt(row?.dataset.id));
  if (!rating) return;
  if (e.target.classList.contains("rating-agency")) rating.agency = e.target.value;
  if (e.target.classList.contains("rating-label")) rating.label = e.target.value;
  refreshIsdcfPreview();
});

document.getElementById("prop-rating-list")?.addEventListener("click", (e) => {
  if (!e.target.classList.contains("rating-remove")) return;
  const row = e.target.closest(".rating-row");
  const index = ratings.findIndex(r => r.id === parseInt(row?.dataset.id));
  if (index < 0) return;
  ratings.splice(index, 1);
  renderRatings();
});

function namingMetadata() {
  return {
    audioLanguage: document.getElementById("prop-audio-language")?.value || null,
    studio: document.getElementById("prop-studio")?.value || null,
    territoryType: document.getElementById("prop-territory-type")?.value || "specific",
    contentVersions: document.getElementById("prop-content-versions")?.value || null,
    ratings: ratings
      .filter(rating => rating.agency.trim() && rating.label.trim())
      .map(rating => ({ agency: rating.agency.trim(), label: rating.label.trim() })),
    tempVersion: document.getElementById("prop-temp-version")?.checked || false,
    preRelease: document.getElementById("prop-pre-release")?.checked || false,
    redBand: document.getElementById("prop-red-band")?.checked || false,
    twoDVersionOfThreeD: document.getElementById("prop-two-d-version-of-three-d")?.checked || false,
    versionFile: document.getElementById("prop-version-file")?.checked || false,
    isdcfNaming: getPrefs().isdcfNaming || false,
  };
}

function compositionMetadata() {
  const text = (id) => document.getElementById(id)?.value?.trim() || null;
  const luminance = text("prop-luminance");
  return {
    versionNumber: text("prop-version-number"),
    chain: text("prop-chain"),
    distributor: text("prop-distributor"),
    facility: text("prop-facility-name"),
    luminance: luminance && `${luminance} ${document.getElementById("prop-luminance-units").value}`,
  };
}

// Everything the built name reads, so the preview and the build agree.
function isdcfNameRequest() {
  const reel = project.reels[0];
  return {
    title: document.getElementById("prop-title")?.value?.trim() || "",
    standard: document.getElementById("prop-standard")?.value || "smpte",
    resolution: document.getElementById("prop-resolution")?.value || "auto",
    framerate: document.getElementById("prop-framerate")?.value || String(DEFAULT_FRAMERATE),
    contentKind: document.getElementById("prop-content-kind")?.value || "feature",
    videoPath: reel?.picture?.path || null,
    audioPath: reel?.sound?.path || null,
    audioChannelDir: document.getElementById("prop-audio-channel-dir")?.value || null,
    audioMap: audioMapSpec(),
    upmix: document.getElementById("prop-upmix")?.value || "none",
    subtitle: reel?.subtitle?.path || null,
    subtitleLanguage: document.getElementById("prop-subtitle-language")?.value || "en",
    burnSubtitle: document.getElementById("prop-burn-subtitle")?.value || null,
    ccap: document.getElementById("prop-ccap")?.value || null,
    ccapLanguage: document.getElementById("prop-ccap-language")?.value || "en",
    rightEye: document.getElementById("prop-right-eye")?.value || null,
    atmos: document.getElementById("prop-atmos")?.value || null,
    facility: getPrefs().facility || null,
    compositionMetadata: compositionMetadata(),
    naming: namingMetadata(),
    sourceWidth: reel?.picture?.width || null,
    sourceHeight: reel?.picture?.height || null,
    cropLeft: parseInt(document.getElementById("prop-crop-left")?.value) || 0,
    cropRight: parseInt(document.getElementById("prop-crop-right")?.value) || 0,
    cropTop: parseInt(document.getElementById("prop-crop-top")?.value) || 0,
    cropBottom: parseInt(document.getElementById("prop-crop-bottom")?.value) || 0,
    rotate: document.getElementById("prop-rotate")?.value || "none",
  };
}

async function refreshIsdcfPreview() {
  const preview = document.getElementById("prop-isdcf-preview");
  if (!preview) return;
  const request = isdcfNameRequest();
  if (!request.naming.isdcfNaming || !request.title) {
    preview.hidden = true;
    return;
  }
  preview.hidden = false;
  try {
    preview.textContent = "ISDCF name: " + await invoke("isdcf_name_preview", { request });
  } catch (e) {
    preview.textContent = String(e);
  }
}

const ISDCF_PREVIEW_CONTROLS = [
  "prop-title", "prop-content-kind", "prop-standard", "prop-resolution", "prop-framerate",
  "prop-subtitle-language", "prop-ccap-language", "prop-audio-language", "prop-studio",
  "prop-territory-type", "prop-content-versions", "prop-temp-version", "prop-pre-release",
  "prop-red-band", "prop-two-d-version-of-three-d", "prop-version-file",
  "prop-crop-left", "prop-crop-right", "prop-crop-top", "prop-crop-bottom", "prop-rotate",
  "prop-audio-channel-dir", "prop-audio-map", "prop-upmix",
  "prop-version-number", "prop-chain", "prop-facility-name", "prop-luminance", "prop-luminance-units",
];
for (const id of ISDCF_PREVIEW_CONTROLS) {
  document.getElementById(id)?.addEventListener("input", refreshIsdcfPreview);
}

renderRatings();

document.getElementById("btn-build")?.addEventListener("click", async () => {
  let title = document.getElementById("prop-title")?.value?.trim();
  if (!title) { tauriMessage("Enter a project title in Properties"); return; }
  // builds are tracked by the Properties title, result.title can differ
  const buildTitle = title;
  if (buildsInFlight.buildInFlight(buildTitle)) {
    tauriMessage(`A build titled "${buildTitle}" is already queued or running. A different title queues another build.`);
    return;
  }
  const anotherBuildInFlight = buildsInFlight.anyBuildInFlight();

  const reel = project.reels[0];
  if (!reel?.picture) { tauriMessage("Import a video asset first"); return; }

  const encrypt = document.getElementById("prop-encrypt")?.checked || false;
  const keyOut = document.getElementById("prop-key-out")?.value || "";
  if (encrypt && !keyOut) {
    tauriMessage("Encryption is on: choose a Key Output File in Properties. It holds the plaintext content keys, keep it secret and outside the DCP.");
    return;
  }

  const video = reel.picture.path;
  const audio = reel.sound?.path || null;
  const outputEl = document.getElementById("prop-output");
  let output = outputEl?.value || await defaultOutputFolder();
  if (outputEl) outputEl.value = output;

  // Show progress
  const progressSection = document.getElementById("progress-section");
  const progressBar = document.getElementById("progress-bar");
  const stageEl = document.getElementById("progress-stage");
  const statsEl = document.getElementById("progress-stats");
  if (!anotherBuildInFlight) {
    progressSection.style.display = "flex";
    progressBar.value = 0;
    stageEl.textContent = "Queued...";
    statsEl.textContent = "";
    paused = false;
    resetStatusBar();
  }
  let projectRecordPath = null;
  let jobId = null;

  const unlisten = await listen("pipeline-progress", (event) => {
    const p = event.payload;
    if (p.job_id !== jobId) return;
    currentJobId = p.job_id;

    const display = progressDisplay(p);
    if (display.percent === null) progressBar.removeAttribute("value");
    else progressBar.value = display.percent;
    stageEl.textContent = display.stage;
    statsEl.textContent = display.stats;
    setTitleProgress(display.percent, p.stage);

    if (p.stage === "done") {
      setStatus("Build complete");
      setTitleProgress(-1);
      notifyBuildComplete(true, title);
      if (projectRecordPath) addRecentProject(projectRecordPath, title);
      showPostBuildActions(output);
      endBuild(buildTitle);
      unlisten();
      unlistenVal();
    } else if (p.stage === "cancelled") {
      setStatus("Cancelled");
      stageEl.textContent = "Cancelled";
      setTitleProgress(-1);
      endBuild(buildTitle);
      unlisten();
      unlistenVal();
    } else if (p.stage === "error") {
      setStatus("Build failed: " + p.message);
      setTitleProgress(-1);
      notifyBuildComplete(false, title);
      tauriMessage(p.message, { title: "Build failed", kind: "error" });
      endBuild(buildTitle);
      unlisten();
      unlistenVal();
    }
  });

  const unlistenVal = await listen("validation-result", (event) => {
    const v = event.payload;
    if (v.job_id !== jobId) return;
    lastValidation = v;
    const validEl = document.getElementById("status-validation");
    validEl.title = "Click for details";
    if (v.valid) {
      validEl.textContent = "✓ Valid";
      validEl.style.color = "#34d399";
    } else {
      validEl.textContent = `✗ ${(v.errors||[]).length} errors`;
      validEl.style.color = "#ff6b6b";
    }
  });

  try {
    beginBuild(buildTitle);
    const submit = (hintsAccepted) => invoke("submit_job", {
      hintsAccepted,
      videoPath: video,
      title,
      outputDir: output,
      audioPath: audio,
      validate: document.getElementById("prop-validate")?.checked ?? true,
      standard: document.getElementById("prop-standard")?.value || "smpte",
      resolution: document.getElementById("prop-resolution")?.value || "auto",
      framerate: document.getElementById("prop-framerate")?.value || String(DEFAULT_FRAMERATE),
      bandwidth: parseInt(document.getElementById("prop-bandwidth")?.value) || DEFAULT_BANDWIDTH_MBPS,
      qualityPsnr: parseFloat(document.getElementById("prop-quality-psnr")?.value) || null,
      contentKind: document.getElementById("prop-content-kind")?.value || "feature",
      encrypt,
      keyOut: keyOut || null,
      signingCert: document.getElementById("set-signing-cert")?.value || null,
      signingKey: document.getElementById("set-signing-key")?.value || null,
      signingChain: signingChainFrom(getPrefs()),
      rightEye: document.getElementById("prop-right-eye")?.value || null,
      atmos: document.getElementById("prop-atmos")?.value || null,
      subtitle: reel.subtitle?.path || null,
      subtitleLanguage: document.getElementById("prop-subtitle-language")?.value || "en",
      subtitleFontSize: document.getElementById("prop-subtitle-font-size")?.value || null,
      subtitleColour: document.getElementById("prop-subtitle-colour")?.value || null,
      subtitleEffect: document.getElementById("prop-subtitle-effect")?.value || null,
      subtitleEffectColour: document.getElementById("prop-subtitle-effect-colour")?.value || null,
      subtitleFadeUp: document.getElementById("prop-subtitle-fade-up")?.value || null,
      subtitleFadeDown: document.getElementById("prop-subtitle-fade-down")?.value || null,
      subtitleHalign: document.getElementById("prop-subtitle-halign")?.value || null,
      subtitleValign: document.getElementById("prop-subtitle-valign")?.value || null,
      subtitleVposition: document.getElementById("prop-subtitle-vposition")?.value || null,
      subtitleZposition: document.getElementById("prop-subtitle-zposition")?.value || null,
      subtitleRtl: document.getElementById("prop-subtitle-rtl")?.value || null,
      subtitleWrap: document.getElementById("prop-subtitle-wrap")?.value || null,
      subtitleFont: document.getElementById("prop-subtitle-font")?.value || null,
      subtitleNoSubset: document.getElementById("prop-subtitle-no-subset")?.checked || false,
      burnSubtitle: document.getElementById("prop-burn-subtitle")?.value || null,
      burnSubtitleFont: document.getElementById("prop-burn-subtitle-font")?.value || null,
      burnFontSize: document.getElementById("prop-burn-font-size")?.value || null,
      burnColour: document.getElementById("prop-burn-colour")?.value || null,
      burnEffect: document.getElementById("prop-burn-effect")?.value || null,
      burnEffectColour: document.getElementById("prop-burn-effect-colour")?.value || null,
      burnOutlineWidth: document.getElementById("prop-burn-outline-width")?.value || null,
      burnLineHeight: document.getElementById("prop-burn-line-height")?.value || null,
      burnMargin: document.getElementById("prop-burn-margin")?.value || null,
      burnFadeUp: document.getElementById("prop-burn-fade-up")?.value || null,
      burnFadeDown: document.getElementById("prop-burn-fade-down")?.value || null,
      ccap: document.getElementById("prop-ccap")?.value || null,
      ccapLanguage: document.getElementById("prop-ccap-language")?.value || "en",
      loudnessTarget: document.getElementById("prop-loudness")?.value || null,
      truePeakCeiling: parseFloat(document.getElementById("prop-true-peak")?.value) || null,
      audioChannelDir: document.getElementById("prop-audio-channel-dir")?.value || null,
      audioMap: audioMapSpec(),
      audioInputOrder: document.getElementById("prop-audio-input-order")?.value || "dcp",
      audioChannels: parseInt(document.getElementById("prop-audio-channels")?.value) || null,
      signLanguageVideo: document.getElementById("prop-sign-language-video")?.value || null,
      signLanguageTag: document.getElementById("prop-sign-language-tag")?.value || null,
      padHead: document.getElementById("prop-pad-head")?.value || null,
      padTail: document.getElementById("prop-pad-tail")?.value || null,
      padColor: document.getElementById("prop-pad-color")?.value || null,
      audioDelayMs: parseInt(document.getElementById("prop-audio-delay")?.value) || 0,
      trimStart: document.getElementById("prop-trim-start")?.value || null,
      trimEnd: document.getElementById("prop-trim-end")?.value || null,
      stillLength: document.getElementById("prop-still-length")?.value || null,
      sourceColourspace: document.getElementById("prop-source-colourspace")?.value || "rec709",
      cropLeft: parseInt(document.getElementById("prop-crop-left")?.value) || 0,
      cropRight: parseInt(document.getElementById("prop-crop-right")?.value) || 0,
      cropTop: parseInt(document.getElementById("prop-crop-top")?.value) || 0,
      cropBottom: parseInt(document.getElementById("prop-crop-bottom")?.value) || 0,
      fillCrop: document.getElementById("prop-fill-crop")?.checked || false,
      deinterlace: document.getElementById("prop-deinterlace")?.checked || false,
      denoise: document.getElementById("prop-denoise")?.checked || false,
      rotate: document.getElementById("prop-rotate")?.value || "none",
      flip: document.getElementById("prop-flip")?.value || "none",
      upmix: document.getElementById("prop-upmix")?.value || "none",
      reelLengthMinutes: parseInt(document.getElementById("prop-reel-length")?.value) || 0,
      splitAt: document.getElementById("prop-split-at")?.value || null,
      splitChapters: document.getElementById("prop-split-chapters")?.checked || false,
      versions: document.getElementById("prop-versions")?.value || null,
      markers: markerSpecs(markerRows),
      hdrDci: document.getElementById("prop-hdr-dci")?.checked || false,
      hdrSource: document.getElementById("prop-hdr-source")?.value || "auto",
      hdrPeakNits: parseFloat(document.getElementById("prop-hdr-peak-nits")?.value) || null,
      hdrToDciLut: document.getElementById("prop-hdr-lut")?.value || null,
      hdrAlreadyPq: document.getElementById("prop-hdr-already-pq")?.checked || false,
      allowGenericHdrTonemap: document.getElementById("prop-hdr-generic-tonemap")?.checked || false,
      facility: getPrefs().facility || null,
      naming: namingMetadata(),
      compositionMetadata: compositionMetadata(),
      headItems: joinedItems.head,
      tailItems: joinedItems.tail,
    });
    let result = await submit(!getPrefs().showHintsBeforeBuild);
    if (result.jobId === null) {
      if (!await showHintsDialog(result.hints)) {
        if (!anotherBuildInFlight) {
          progressSection.style.display = "none";
          setStatus("Build cancelled");
        }
        endBuild(buildTitle);
        unlisten();
        unlistenVal();
        return;
      }
      result = await submit(true);
    }
    title = result.title;
    output = result.outputDir;
    submittedPackage = output;
    jobId = result.jobId;
    if (anotherBuildInFlight) {
      setStatus(`Queued "${buildTitle}" behind the running build`);
    } else {
      currentJobId = jobId;
      setStatus("Building DCP...");
    }
    projectRecordPath = await saveProjectBesidePackage(output);
  } catch (e) {
    if (!anotherBuildInFlight) {
      stageEl.textContent = "Failed";
      setStatus("Error: " + e);
    }
    tauriMessage(String(e), { title: "Build failed", kind: "error" });
    endBuild(buildTitle);
    unlisten();
    unlistenVal();
  }
});

// Cancel button in progress bar
document.getElementById("progress-cancel")?.addEventListener("click", async () => {
  if (currentJobId) {
    await invoke("cancel_job", { jobId: currentJobId });
    setStatus("Cancelled");
  }
});

// === Preview ===
const CROP_SIDES = ["left", "right", "top", "bottom"];

// mpv refuses a subtitle track until the clip it goes on has loaded, and the
// load command only asks for the load, so a duration is the signal it landed.
const PREVIEW_LOAD_POLL_MILLISECONDS = 100;
const PREVIEW_LOAD_POLL_ATTEMPTS = 30;

// A preview opened while an earlier one's subtitles are still converting must
// not be given those tracks.
let previewGeneration = 0;
let previewShowsJobPicture = false;
// the package last opened, which the Preview button plays when no picture is imported
let openedPackage = null;
// the package folder of the last build submitted
let submittedPackage = null;

// the row picked in the asset list or the recent list
let selectedPreview = null;

// what the preview panel holds
let previewShownPath = null;

function selectPreview(kind, path) {
  selectedPreview = { kind, path };
  applyPreviewSelection();
  updateToolbarState();
}

function clearPreviewSelection() {
  selectedPreview = null;
  applyPreviewSelection();
  updateToolbarState();
}

// both lists rebuild their rows from innerHTML
function applyPreviewSelection() {
  const selectedAssetId = selectedPreview?.kind === "source"
    ? project.assets.find(a => a.path === selectedPreview.path)?.id
    : null;
  document.querySelectorAll("#asset-list .asset-item").forEach(el => {
    el.classList.toggle("selected", parseInt(el.dataset.assetId) === selectedAssetId);
  });
  document.querySelectorAll("#recent-list .recent-item").forEach(el => {
    const isSelected = selectedPreview?.kind === "package" && el.dataset.packagePath === selectedPreview.path;
    el.classList.toggle("selected", isSelected);
  });
}

function previewTargetInput() {
  return {
    selectedPreview,
    firstPicturePath: project.reels[0]?.picture?.path,
    openedPackage,
    outputPath: submittedPackage,
  };
}

document.getElementById("btn-preview")?.addEventListener("click", () => {
  const target = previewTarget(previewTargetInput());
  if (!target) return;
  if (target.kind === PREVIEW_KIND_SOURCE) previewSourcePicture(target.path);
  else previewPackage(target.path);
});

// content keys entered this session, by the package or picture path they opened
const sessionContentKeys = new Map();

// contentKeys is null for a path that is not encrypted
async function contentKeysFor(path) {
  let encrypted;
  try {
    encrypted = await previewNeedsContentKeys(path);
  } catch (e) {
    setStatus(`Preview failed: ${e}`);
    return { cancelled: true };
  }
  if (!encrypted) return { contentKeys: null };
  const remembered = sessionContentKeys.get(path);
  if (remembered) return { contentKeys: remembered };
  const contentKeys = await askForContentKeys();
  if (!contentKeys) return { cancelled: true };
  sessionContentKeys.set(path, contentKeys);
  return { contentKeys };
}

// keys that failed to load are asked for again next time
function forgetContentKeysIfRefused(path, loaded) {
  loaded.then((worked) => {
    if (!worked) sessionContentKeys.delete(path);
  });
}

const CONTENT_KEY_FIELDS = [
  { field: "kdm", input: "content-keys-kdm", browse: "content-keys-browse-kdm" },
  { field: "recipientKey", input: "content-keys-recipient-key", browse: "content-keys-browse-recipient-key" },
  { field: "keys", input: "content-keys-keys", browse: "content-keys-browse-keys" },
];

// a recipient key without a KDM is refused
function prefillRecipientKeyBesideKdm(recipientKeyInput) {
  const input = document.getElementById(recipientKeyInput);
  input.value = prefilledRecipientKey(getPrefs(), input.value);
}

for (const { field, input, browse } of CONTENT_KEY_FIELDS) {
  document.getElementById(browse)?.addEventListener("click", async () => {
    const file = await open({ directory: false });
    if (!file) return;
    document.getElementById(input).value = file;
    if (field === "kdm") prefillRecipientKeyBesideKdm("content-keys-recipient-key");
  });
}

function askForContentKeys() {
  const dialog = document.getElementById("content-keys-dialog");
  const refusalList = document.getElementById("content-keys-refusals");
  if (!dialog || !refusalList) return Promise.resolve(null);
  for (const { input } of CONTENT_KEY_FIELDS) document.getElementById(input).value = "";
  refusalList.innerHTML = "";
  coverPreviewSurface();
  dialog.hidden = false;

  return new Promise((resolve) => {
    const playButton = document.getElementById("content-keys-play");
    const cancelButton = document.getElementById("content-keys-cancel");
    const close = (contentKeys) => {
      dialog.hidden = true;
      uncoverPreviewSurface();
      playButton.removeEventListener("click", onPlay);
      cancelButton.removeEventListener("click", onCancel);
      resolve(contentKeys);
    };
    const onPlay = () => {
      const fields = {};
      for (const { field, input } of CONTENT_KEY_FIELDS) fields[field] = document.getElementById(input).value;
      const { contentKeys, refusals } = contentKeysFrom(fields);
      if (contentKeys) {
        close(contentKeys);
        return;
      }
      refusalList.innerHTML = "";
      for (const refusal of refusals) {
        const item = document.createElement("li");
        item.textContent = refusal;
        refusalList.appendChild(item);
      }
    };
    const onCancel = () => close(null);
    playButton.addEventListener("click", onPlay);
    cancelButton.addEventListener("click", onCancel);
  });
}

// The preview shows what the build will do to the picture, so a file a reel
// takes as its picture carries the crop and that reel's timed text, and any
// other file plays plain.
async function previewSourcePicture(path) {
  const { contentKeys, cancelled } = await contentKeysFor(path);
  if (cancelled) return;
  previewGeneration += 1;
  const generation = previewGeneration;
  forgetContentKeysIfRefused(path, previewFile(path, contentKeys));
  const reel = project.reels.find(r => r.picture?.path === path);
  previewShowsJobPicture = Boolean(reel);
  setPreviewCrop(reel ? currentCrop() : null);
  if (!reel) return;
  showPreviewTrack(reel.subtitle?.path, "subtitle", generation);
  showPreviewTrack(document.getElementById("prop-ccap")?.value, "closed-caption", generation);
}

// A built DCP carries the crop in its pictures already, and its timed text is
// read back out of the package rather than off the source files.
async function previewPackage(dirPath) {
  const { contentKeys, cancelled } = await contentKeysFor(dirPath);
  if (cancelled) return;
  previewGeneration += 1;
  const generation = previewGeneration;
  previewShowsJobPicture = false;
  forgetContentKeysIfRefused(dirPath, previewDcp(dirPath, contentKeys));
  setPreviewCrop(null);
  showPreviewTrack(dirPath, "subtitle", generation);
  showPreviewTrack(dirPath, "closed-caption", generation);
}

function currentCrop() {
  const crop = {};
  for (const side of CROP_SIDES) {
    crop[side] = parseInt(document.getElementById(`prop-crop-${side}`)?.value) || 0;
  }
  return CROP_SIDES.some(side => crop[side] > 0) ? crop : null;
}

function refreshPreviewCrop() {
  if (previewShowsJobPicture && isPreviewVisible()) setPreviewCrop(currentCrop());
}

for (const side of CROP_SIDES) {
  document.getElementById(`prop-crop-${side}`)?.addEventListener("input", refreshPreviewCrop);
}

async function showPreviewTrack(sourcePath, track, generation) {
  if (!sourcePath) return;
  let playable;
  try {
    playable = await invoke("subtitle_file_for_preview", {
      sourcePath,
      track,
      fps: parseInt(document.getElementById("prop-framerate")?.value) || DEFAULT_FRAMERATE,
    });
  } catch (e) {
    console.error(`[preview] ${track} not shown:`, e);
    setStatus(`Preview ${track}: ${e}`);
    return;
  }
  const loaded = await previewClipLoaded();
  if (!loaded || generation !== previewGeneration) return;
  if (track === "subtitle") setPreviewSubtitleFile(playable);
  else setPreviewCaptionFile(playable);
}

async function previewClipLoaded() {
  for (let attempt = 0; attempt < PREVIEW_LOAD_POLL_ATTEMPTS; attempt++) {
    const duration = await invoke("preview_get_duration").catch(() => 0);
    if (duration > 0) return true;
    await new Promise(resolve => setTimeout(resolve, PREVIEW_LOAD_POLL_MILLISECONDS));
  }
  return false;
}

// === Post-build actions ===
function finishedOutputDir() {
  return document.getElementById("post-build-actions")?.dataset.output;
}

function recentTitleFor(packagePath) {
  const projectPath = projectPathBeside(packagePath);
  return getRecentProjects().find((r) => r.path === projectPath)?.title;
}

function showPostBuildActions(outputDir) {
  const row = document.getElementById("post-build-actions");
  if (!row) return;
  row.dataset.output = outputDir;
  row.hidden = false;
}

function hidePostBuildActions() {
  const row = document.getElementById("post-build-actions");
  if (row) row.hidden = true;
}

document.getElementById("post-build-play")?.addEventListener("click", () => {
  const output = finishedOutputDir();
  if (output) previewPackage(output);
});

document.getElementById("post-build-queue")?.addEventListener("click", () => {
  const output = finishedOutputDir();
  if (output) addToPlaylist(output, recentTitleFor(output));
});

document.getElementById("post-build-inspect")?.addEventListener("click", () => {
  const output = finishedOutputDir();
  if (!output) return;
  switchView("verify");
  document.getElementById("verify-path").textContent = output;
  document.getElementById("verify-run").disabled = false;
  runVerification();
});

document.getElementById("post-build-reveal")?.addEventListener("click", () => {
  const output = finishedOutputDir();
  if (output) revealItemInDir(output);
});

// === Verify ===
document.getElementById("verify-browse")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (!dir) return;
  document.getElementById("verify-path").textContent = dir;
  document.getElementById("verify-run").disabled = false;
  const resultsBox = document.getElementById("verify-results");
  resultsBox.textContent = "";
  resultsBox.classList.remove("visible");
});

async function runVerification() {
  const dir = document.getElementById("verify-path").textContent;
  if (!dir || dir.startsWith("No ")) return;

  const resultsBox = document.getElementById("verify-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Verifying...";

  const args = ["verify", dir, "--strict"];
  if (!document.getElementById("verify-mxf")?.checked) args.push("--no-picture-check");
  if (!document.getElementById("verify-hashes")?.checked) args.push("--no-hash-check");

  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  if (result.code === 0) {
    resultsBox.textContent = "✓ DCP verification PASSED\n\n" + result.stdout;
    setStatus("Verification passed");
  } else {
    resultsBox.textContent = "✗ Verification failed\n\n" + result.stdout + result.stderr;
    setStatus("Verification failed");
  }
}

document.getElementById("verify-run")?.addEventListener("click", runVerification);

// === Encryption & KDM ===
function fieldValue(id) {
  return document.getElementById(id)?.value || "";
}

function signerFields() {
  const prefs = getPrefs();
  return {
    signerCert: prefs.signingCert,
    signerKey: prefs.signingKey,
    signerIntermediate: prefs.signingIntermediate,
    signerRoot: prefs.signingRoot,
    offsetMinutes: -new Date().getTimezoneOffset(),
  };
}

function refreshSignerLine() {
  const line = document.getElementById("kdm-signer-line");
  if (line) line.textContent = describeSigner(getPrefs());
  checkKdmReady();
  checkRewrapReady();
}

function kdmFields() {
  const prefs = getPrefs();
  return {
    ...signerFields(),
    cplId: fieldValue("kdm-cpl-id").trim(),
    contentTitle: fieldValue("kdm-content-title").trim(),
    forThisMachine: document.getElementById("kdm-for-this-machine")?.checked || false,
    recipientCert: fieldValue("kdm-cert"),
    recipientFromSettings: prefs.recipientCert,
    keys: fieldValue("kdm-keys"),
    validFrom: fieldValue("kdm-from"),
    validTo: fieldValue("kdm-to"),
    formulation: fieldValue("kdm-formulation"),
    noForensicPicture: document.getElementById("kdm-no-forensic-picture")?.checked || false,
    audioMarking: fieldValue("kdm-audio-marking") || "on",
    audioMarkingChannel: fieldValue("kdm-audio-marking-channel"),
    output: fieldValue("kdm-output"),
  };
}

function rewrapFields() {
  return {
    ...signerFields(),
    dkdm: fieldValue("dkdm-file"),
    dkdmKey: fieldValue("dkdm-key"),
    recipientCert: fieldValue("dkdm-cert"),
    validFrom: fieldValue("dkdm-from"),
    validTo: fieldValue("dkdm-to"),
    formulation: fieldValue("dkdm-formulation"),
    noForensicPicture: document.getElementById("dkdm-no-forensic-picture")?.checked || false,
    audioMarking: fieldValue("dkdm-audio-marking") || "on",
    audioMarkingChannel: fieldValue("dkdm-audio-marking-channel"),
    output: fieldValue("dkdm-output"),
  };
}

function checkKdmReady() {
  const btn = document.getElementById("run-kdm");
  if (btn) btn.disabled = kdmRequest(kdmFields()).refusals.length > 0;
}

function checkRewrapReady() {
  const btn = document.getElementById("run-rewrap");
  if (btn) btn.disabled = rewrapRequest(rewrapFields()).refusals.length > 0;
}

function applyCplIdentity(identity) {
  document.getElementById("kdm-cpl-id").value = identity.id;
  document.getElementById("kdm-content-title").value = identity.title;
  checkKdmReady();
}

function showCplChoices(identities) {
  const field = document.getElementById("kdm-cpl-pick-field");
  const select = document.getElementById("kdm-cpl-pick");
  if (!field || !select) return;
  select.innerHTML = "";
  if (identities.length < 2) {
    field.hidden = true;
    return;
  }
  for (const identity of identities) {
    const option = document.createElement("option");
    option.value = identity.id;
    option.textContent = identity.title || identity.id;
    option.dataset.title = identity.title;
    select.appendChild(option);
  }
  field.hidden = false;
}

function useOwnKey(checkboxId, inputId, browseId, preference) {
  const checkbox = document.getElementById(checkboxId);
  const input = document.getElementById(inputId);
  const browse = document.getElementById(browseId);
  if (!checkbox || !input) return;
  const own = checkbox.checked;
  input.readOnly = true;
  if (browse) browse.disabled = own;
  if (own) input.value = preference || "";
}

document.getElementById("kdm-browse-dcp")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (!dir) return;
  document.getElementById("kdm-dcp").value = dir;
  try {
    const identities = await invoke("cpl_identities", { dcpDir: dir });
    showCplChoices(identities);
    applyCplIdentity(identities[0]);
  } catch (error) {
    showCplChoices([]);
    setStatus(`Could not read a composition from that package: ${error}`);
    checkKdmReady();
  }
});

document.getElementById("kdm-cpl-pick")?.addEventListener("change", (event) => {
  const option = event.target.selectedOptions[0];
  if (!option) return;
  applyCplIdentity({ id: option.value, title: option.dataset.title || "" });
});

document.getElementById("kdm-browse-cert")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) { document.getElementById("kdm-cert").value = file; checkKdmReady(); }
});
document.getElementById("kdm-browse-keys")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) { document.getElementById("kdm-keys").value = file; checkKdmReady(); }
});
document.getElementById("kdm-browse-output")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (!dir) return;
  const name = document.getElementById("kdm-for-this-machine")?.checked ? "dkdm.xml" : "kdm.xml";
  document.getElementById("kdm-output").value = `${dir}/${name}`;
  checkKdmReady();
});

for (const id of ["kdm-cpl-id", "kdm-content-title", "kdm-from", "kdm-to", "kdm-formulation", "kdm-audio-marking", "kdm-audio-marking-channel"]) {
  document.getElementById(id)?.addEventListener("input", checkKdmReady);
  document.getElementById(id)?.addEventListener("change", checkKdmReady);
}
document.getElementById("kdm-no-forensic-picture")?.addEventListener("change", checkKdmReady);
document.getElementById("kdm-for-this-machine")?.addEventListener("change", () => {
  useOwnKey("kdm-for-this-machine", "kdm-cert", "kdm-browse-cert", getPrefs().recipientCert);
  checkKdmReady();
});

document.getElementById("run-kdm")?.addEventListener("click", async () => {
  const request = kdmRequest(kdmFields());
  const resultsBox = document.getElementById("kdm-results");
  resultsBox.classList.add("visible");
  if (request.refusals.length) {
    resultsBox.textContent = request.refusals.join("\n");
    return;
  }
  resultsBox.textContent = "Making KDM…";
  const result = await Command.sidecar("dcpwizard", request.args).execute();
  resultsBox.textContent = result.code === 0
    ? "KDM written\n\n" + result.stdout
    : "Failed\n\n" + (result.stderr || result.stdout);
});

document.getElementById("dkdm-browse")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) { document.getElementById("dkdm-file").value = file; checkRewrapReady(); }
});
document.getElementById("dkdm-browse-key")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) { document.getElementById("dkdm-key").value = file; checkRewrapReady(); }
});
document.getElementById("dkdm-browse-cert")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) { document.getElementById("dkdm-cert").value = file; checkRewrapReady(); }
});
document.getElementById("dkdm-browse-output")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("dkdm-output").value = `${dir}/kdm.xml`; checkRewrapReady(); }
});
for (const id of ["dkdm-from", "dkdm-to", "dkdm-formulation", "dkdm-audio-marking", "dkdm-audio-marking-channel"]) {
  document.getElementById(id)?.addEventListener("input", checkRewrapReady);
  document.getElementById(id)?.addEventListener("change", checkRewrapReady);
}
document.getElementById("dkdm-no-forensic-picture")?.addEventListener("change", checkRewrapReady);
document.getElementById("dkdm-use-own-key")?.addEventListener("change", () => {
  useOwnKey("dkdm-use-own-key", "dkdm-key", "dkdm-browse-key", getPrefs().recipientKey);
  checkRewrapReady();
});

document.getElementById("run-rewrap")?.addEventListener("click", async () => {
  const request = rewrapRequest(rewrapFields());
  const resultsBox = document.getElementById("rewrap-results");
  resultsBox.classList.add("visible");
  if (request.refusals.length) {
    resultsBox.textContent = request.refusals.join("\n");
    return;
  }
  resultsBox.textContent = "Making KDM…";
  const result = await Command.sidecar("dcpwizard", request.args).execute();
  resultsBox.textContent = result.code === 0
    ? "KDM written\n\n" + result.stdout
    : "Failed\n\n" + (result.stderr || result.stdout);
});

// === Version File (Supplemental DCP) ===
let vfNextId = 1;
const vfReplacements = [{ id: vfNextId++, reel: 1, picture: "", sound: "" }];

function renderVfReplacements() {
  const list = document.getElementById("vf-repl-list");
  if (!list) return;
  list.innerHTML = vfReplacements.map(r => `
    <div class="field-row vf-repl-row" data-id="${r.id}">
      <div class="prop-field vf-reel-field">
        <label>Reel</label>
        <input type="number" class="vf-reel-num" min="1" value="${r.reel}">
      </div>
      <div class="prop-field">
        <label>Picture (optional)</label>
        <div class="input-with-btn">
          <input type="text" class="vf-picture" value="${r.picture}" placeholder="J2K dir or .mxf…" readonly>
          <button class="btn-sm vf-browse-picture" type="button">…</button>
        </div>
      </div>
      <div class="prop-field">
        <label>Sound (optional)</label>
        <div class="input-with-btn">
          <input type="text" class="vf-sound" value="${r.sound}" placeholder="WAV or .mxf…" readonly>
          <button class="btn-sm vf-browse-sound" type="button">…</button>
        </div>
      </div>
      <button class="btn-sm vf-remove" type="button" title="Remove reel" ${vfReplacements.length <= 1 ? "disabled" : ""}>✕</button>
    </div>
  `).join("");
  checkVfReady();
}

function checkVfReady() {
  const btn = document.getElementById("vf-create");
  if (!btn) return;
  const ok = document.getElementById("vf-ov")?.value &&
    document.getElementById("vf-output")?.value &&
    vfReplacements.some(r => r.picture || r.sound);
  btn.disabled = !ok;
}

document.getElementById("vf-browse-ov")?.addEventListener("click", async () => {
  const d = await open({ directory: true });
  if (d) { document.getElementById("vf-ov").value = d; checkVfReady(); }
});
document.getElementById("vf-browse-output")?.addEventListener("click", async () => {
  const d = await open({ directory: true });
  if (d) { document.getElementById("vf-output").value = d; checkVfReady(); }
});
document.getElementById("vf-title")?.addEventListener("input", checkVfReady);

document.getElementById("vf-add-repl")?.addEventListener("click", () => {
  const maxReel = vfReplacements.reduce((m, r) => Math.max(m, parseInt(r.reel) || 0), 0);
  vfReplacements.push({ id: vfNextId++, reel: maxReel + 1, picture: "", sound: "" });
  renderVfReplacements();
});

// Per-row picker / remove via delegation (rows are re-rendered).
document.getElementById("vf-repl-list")?.addEventListener("click", async (e) => {
  const row = e.target.closest(".vf-repl-row");
  if (!row) return;
  const rec = vfReplacements.find(r => r.id === parseInt(row.dataset.id));
  if (!rec) return;
  if (e.target.classList.contains("vf-browse-picture")) {
    const d = await open({ directory: true });
    if (d) { rec.picture = d; renderVfReplacements(); }
  } else if (e.target.classList.contains("vf-browse-sound")) {
    const f = await open({ directory: false, filters: [{ name: "Audio/MXF", extensions: ["wav", "mxf"] }, { name: "All", extensions: ["*"] }] });
    if (f) { rec.sound = f; renderVfReplacements(); }
  } else if (e.target.classList.contains("vf-remove")) {
    if (vfReplacements.length <= 1) return;
    vfReplacements.splice(vfReplacements.findIndex(r => r.id === rec.id), 1);
    renderVfReplacements();
  }
});
// Reel number edits shouldn't re-render (would drop focus).
document.getElementById("vf-repl-list")?.addEventListener("input", (e) => {
  if (!e.target.classList.contains("vf-reel-num")) return;
  const row = e.target.closest(".vf-repl-row");
  const rec = vfReplacements.find(r => r.id === parseInt(row.dataset.id));
  if (rec) rec.reel = e.target.value;
});

document.getElementById("vf-create")?.addEventListener("click", async () => {
  const ov = document.getElementById("vf-ov")?.value;
  const output = document.getElementById("vf-output")?.value;
  const title = document.getElementById("vf-title")?.value?.trim();
  if (!ov || !output) { tauriMessage("Select the OV and output directories"); return; }

  const replacements = vfReplacements
    .map(r => ({ reel_number: parseInt(r.reel) || 0, picture: r.picture || null, sound: r.sound || null }))
    .filter(r => r.picture || r.sound);
  if (replacements.length === 0) { tauriMessage("Add at least one reel with a picture or sound"); return; }

  const box = document.getElementById("vf-results");
  box.classList.add("visible");
  box.textContent = "Creating Version File DCP…";
  setStatus("Creating Version File…");
  try {
    const packageDir = await invoke("create_vf", { ovDir: ov, outputDir: output, title: title || null, replacements });
    box.textContent = `✓ Created Version File DCP at ${packageDir}`;
    setStatus("Version File created");
  } catch (e) {
    box.textContent = "✗ " + e;
    setStatus("Version File failed: " + e);
  }
});

renderVfReplacements();

// === Export DCP ===
const EXPORT_FIELD_IDS = {
  input: "export-input",
  output: "export-output",
  format: "export-format",
  crf: "export-crf",
  audio: "export-audio",
  kdm: "export-kdm",
  recipientKey: "export-recipient-key",
  keys: "export-keys",
};

function exportFields() {
  return Object.fromEntries(
    Object.entries(EXPORT_FIELD_IDS).map(([field, id]) => [field, document.getElementById(id).value]),
  );
}

async function browseExportFile(fieldId, filters) {
  const path = await open({ directory: false, multiple: false, filters });
  if (path) document.getElementById(fieldId).value = path;
}

function showExportResult(text, revealedPath) {
  const box = document.getElementById("export-results");
  const reveal = document.getElementById("export-reveal");
  box.classList.add("visible");
  document.getElementById("export-result-text").textContent = text;
  reveal.hidden = !revealedPath;
  reveal.dataset.path = revealedPath || "";
}

function setExportRunning(running) {
  document.getElementById("export-start").disabled = running;
  document.getElementById("export-cancel").disabled = !running;
}

document.getElementById("export-browse-folder")?.addEventListener("click", async () => {
  const directory = await open({ directory: true });
  if (directory) document.getElementById("export-input").value = directory;
});
document.getElementById("export-browse-file")?.addEventListener("click", () =>
  browseExportFile("export-input", [{ name: "CPL or MXF", extensions: ["xml", "mxf"] }]));
document.getElementById("export-browse-audio")?.addEventListener("click", () =>
  browseExportFile("export-audio", [{ name: "Sound", extensions: ["mxf", "wav"] }]));
document.getElementById("export-browse-kdm")?.addEventListener("click", async () => {
  await browseExportFile("export-kdm", [{ name: "KDM", extensions: ["xml"] }, { name: "All", extensions: ["*"] }]);
  if (document.getElementById(EXPORT_FIELD_IDS.kdm).value) prefillRecipientKeyBesideKdm(EXPORT_FIELD_IDS.recipientKey);
});
document.getElementById("export-browse-recipient-key")?.addEventListener("click", () =>
  browseExportFile("export-recipient-key", [{ name: "Private key", extensions: ["pem", "key"] }, { name: "All", extensions: ["*"] }]));
document.getElementById("export-browse-keys")?.addEventListener("click", () =>
  browseExportFile("export-keys", [{ name: "KEYS.json", extensions: ["json"] }]));

document.getElementById("export-format")?.addEventListener("change", (event) => {
  document.getElementById("export-crf-field").hidden = !takesCrf(event.target.value);
});

document.getElementById("export-browse-output")?.addEventListener("click", async () => {
  const format = document.getElementById("export-format").value;
  if (!isMovieFormat(format)) {
    const directory = await open({ directory: true });
    if (directory) document.getElementById("export-output").value = directory;
    return;
  }
  const extensions = movieExtensions(format);
  const file = await save({
    defaultPath: `export.${extensions[0]}`,
    filters: [{ name: "Movie", extensions }],
  });
  if (file) document.getElementById("export-output").value = withMovieExtension(file, format);
});

listen("export-progress", (event) => {
  document.getElementById("export-progress").value = exportProgressPercent(event.payload);
  document.getElementById("export-progress-text").textContent = exportProgressText(event.payload);
});

document.getElementById("export-start")?.addEventListener("click", async () => {
  const { request, refusals } = exportRequestFrom(exportFields());
  if (refusals) {
    showExportResult(refusals.join("\n"));
    return;
  }
  setExportRunning(true);
  document.getElementById("export-progress").value = 0;
  document.getElementById("export-progress-text").textContent = "";
  showExportResult("Exporting…");
  try {
    await invoke("export_dcp", { request });
    showExportResult(`Exported to ${request.output}`, request.output);
  } catch (error) {
    showExportResult(String(error));
  } finally {
    setExportRunning(false);
  }
});

document.getElementById("export-cancel")?.addEventListener("click", () => invoke("export_cancel"));

document.getElementById("export-reveal")?.addEventListener("click", (event) => {
  revealItemInDir(event.target.dataset.path);
});

// === Jobs ===
const DAEMON_JOB_SOURCE = "daemon";
const DAEMON_ONLINE_STATUS = "Online";
const DAEMON_OFFLINE_STATUS = "Offline";
const DAEMON_ERROR_STATUS = "Error";
// `batch list` prints one header line before its jobs.
const DAEMON_JOB_LIST_HEADER_LINES = 1;

function daemonJobRows(stdout) {
  const lines = stdout.trim().split("\n");
  if (lines.length <= 1 || lines[0].startsWith("No jobs")) return [];
  return lines.slice(DAEMON_JOB_LIST_HEADER_LINES).filter(line => line.trim()).map(line => {
    const [id, state, progress, type] = line.trim().split(/\s+/);
    return {
      id,
      label: type,
      state,
      progress,
      message: "",
      cancel: () => Command.sidecar("dcpwizard", ["batch", "cancel", id]).execute(),
      move: (beforeId) => Command.sidecar("dcpwizard", ["batch", "move", id, ...(beforeId === null ? [] : ["--before", beforeId])]).execute(),
    };
  });
}

async function batchDaemonJobs() {
  try {
    const result = await Command.sidecar("dcpwizard", ["batch", "list"]).execute();
    if (result.code !== 0) {
      return { source: DAEMON_JOB_SOURCE, status: DAEMON_OFFLINE_STATUS, rows: [] };
    }
    return {
      source: DAEMON_JOB_SOURCE,
      status: DAEMON_ONLINE_STATUS,
      rows: daemonJobRows(result.stdout),
    };
  } catch {
    return { source: DAEMON_JOB_SOURCE, status: DAEMON_ERROR_STATUS, rows: [] };
  }
}

initJobsPanel({
  tableBody: document.getElementById("jobs-tbody"),
  statusBadge: document.getElementById("jobs-status"),
  refreshButton: document.getElementById("jobs-refresh"),
  extraRows: batchDaemonJobs,
});

// === Status bar ===
function setStatus(text) {
  const el = document.getElementById("status-text");
  if (el) {
    el.textContent = text;
    el.title = text;
  }
}

let lastValidation = null;

function resetStatusBar() {
  setStatus("");
  lastValidation = null;
  const validEl = document.getElementById("status-validation");
  if (validEl) {
    validEl.textContent = "";
    validEl.title = "";
  }
}

document.getElementById("status-validation")?.addEventListener("click", () => {
  if (!lastValidation) return;
  const errors = lastValidation.errors || [];
  const warnings = lastValidation.warnings || [];
  const text = [
    ...errors.map((e) => `ERROR: ${e}`),
    ...warnings.map((w) => `WARNING: ${w}`),
  ].join("\n\n");
  if (!text) return;
  tauriMessage(text, {
    title: "Validation",
    kind: errors.length ? "error" : "warning",
  });
});

// === Free disk ===
const DISK_REFRESH_MS = 30000;
const DISK_LOW_PERCENT = 10;

function formatBytes(bytes) {
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) { value /= 1000; unit++; }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

async function refreshDiskSpace() {
  const el = document.getElementById("status-disk");
  if (!el) return;
  const path = document.getElementById("prop-output")?.value || await defaultOutputFolder();
  let space;
  try {
    space = await invoke("disk_space", { path });
  } catch {
    el.textContent = "";
    el.title = "";
    return;
  }
  const percent = Math.round(space.percent_free);
  el.textContent = `💾 ${percent}%`;
  el.title = `${formatBytes(space.free_bytes)} free of ${formatBytes(space.total_bytes)} on ${path}`;
  el.style.color = percent <= DISK_LOW_PERCENT ? "#ff6b6b" : "";
}

refreshDiskSpace();
setInterval(refreshDiskSpace, DISK_REFRESH_MS);

// === Title sync ===
function setProjectTitle(title) {
  const titleElement = document.getElementById("prop-title");
  titleElement.value = title;
  titleElement.dispatchEvent(new Event("input"));
}

document.getElementById("prop-title")?.addEventListener("input", (e) => {
  const title = e.target.value.trim();
  project.title = title;
});

// === Tools: Encode J2K ===
document.getElementById("enc-browse-input")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("enc-input").value = dir; checkEncodeReady(); }
});
document.getElementById("enc-browse-output")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("enc-output").value = dir; checkEncodeReady(); }
});

function checkEncodeReady() {
  const btn = document.getElementById("run-encode");
  if (btn) btn.disabled = !(document.getElementById("enc-input")?.value && document.getElementById("enc-output")?.value);
}

document.getElementById("run-encode")?.addEventListener("click", async () => {
  const input = document.getElementById("enc-input").value;
  const output = document.getElementById("enc-output").value;
  const resolution = document.getElementById("enc-resolution").value;
  const bandwidth = document.getElementById("enc-bandwidth").value;
  const framerate = document.getElementById("enc-framerate").value;
  const resultsBox = document.getElementById("encode-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Encoding...";
  const args = ["encode", "-i", input, "-o", output, "--bandwidth", bandwidth];
  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? "✓ Encode complete\n\n" + result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Tools: Transcode ===
document.getElementById("tc-browse-input")?.addEventListener("click", async () => {
  const file = await open({ directory: false, filters: [{ name: 'Video', extensions: ['mp4','mkv','mov','avi','mxf','webm'] }, { name: 'All', extensions: ['*'] }] });
  if (file) { document.getElementById("tc-input").value = file; checkTranscodeReady(); }
});
document.getElementById("tc-browse-output")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("tc-output").value = dir; checkTranscodeReady(); }
});

function checkTranscodeReady() {
  const btn = document.getElementById("tc-start");
  if (btn) btn.disabled = !(document.getElementById("tc-input")?.value && document.getElementById("tc-output")?.value);
}

document.getElementById("tc-start")?.addEventListener("click", async () => {
  const input = document.getElementById("tc-input").value;
  const output = document.getElementById("tc-output").value;
  const format = document.getElementById("tc-format").value;
  const bitdepth = document.getElementById("tc-bitdepth").value;
  const resultsBox = document.getElementById("tc-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Transcoding...";
  const args = ["transcode", "-i", input, "-o", output, "--format", format, "--bit-depth", bitdepth];
  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? "✓ Transcode complete\n\n" + result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Tools: Loudness ===
document.getElementById("loud-browse")?.addEventListener("click", async () => {
  const file = await open({ directory: false, filters: [{ name: 'Audio', extensions: ['wav','aiff','flac','mp3'] }, { name: 'All', extensions: ['*'] }] });
  if (file) { document.getElementById("loud-input").value = file; document.getElementById("loud-measure").disabled = false; }
});

document.getElementById("loud-measure")?.addEventListener("click", async () => {
  const input = document.getElementById("loud-input").value;
  const resultsBox = document.getElementById("loud-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Measuring loudness...";
  const cmd = Command.sidecar("dcpwizard", ["loudness", input]);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? "✓ Loudness measured\n\n" + result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Tools: Copy DCP ===
document.getElementById("copy-browse-source")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("copy-source").value = dir; checkCopyReady(); }
});
document.getElementById("copy-browse-dest")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("copy-dest").value = dir; checkCopyReady(); }
});

function checkCopyReady() {
  const btn = document.getElementById("copy-start");
  if (btn) btn.disabled = !(document.getElementById("copy-source")?.value && document.getElementById("copy-dest")?.value);
}

document.getElementById("copy-start")?.addEventListener("click", async () => {
  const source = document.getElementById("copy-source").value;
  const dest = document.getElementById("copy-dest").value;
  const resultsBox = document.getElementById("copy-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Copying...";
  const args = ["copy", "--src", source, "--dst", dest];
  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? "✓ Copy complete\n\n" + result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Tools: QC Report ===
document.getElementById("report-browse")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("report-dcp").value = dir; document.getElementById("report-start").disabled = false; }
});

document.getElementById("report-start")?.addEventListener("click", async () => {
  const dcp = document.getElementById("report-dcp").value;
  const format = document.getElementById("report-format").value;
  const resultsBox = document.getElementById("report-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Generating report...";
  const args = ["report", "--dcp", dcp, "-o", dcp + "/report." + format];
  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Projects ===
function buildPanel() {
  return { elementById: (id) => document.getElementById(id), project, markerRows, ratings, joinedItems };
}

function serializeBuildPanel() {
  return serializeForm({ ...buildPanel(), audioMap: audioMapSpec() });
}

async function restoreBuildPanel(saved) {
  const { form, notRestored } = restoreFormState(saved, buildPanelDefaults, buildPanel());
  nextAssetId = Math.max(0, ...project.assets.map((asset) => asset.id)) + 1;
  nextCplId = Math.max(0, ...project.compositions.map((composition) => composition.id)) + 1;
  markerNextId = markerRows.length + 1;
  ratingNextId = ratings.length + 1;
  document.getElementById("prop-profile").value = "";
  applyProfile("");

  const labels = await markerLabelsRequest;
  document.getElementById("prop-markers").replaceChildren(...markerRows.map((row) => markerRowElement(row, labels)));
  renderRatings();
  renderCplTabs();
  // undefined matches no sound path, not even none, so the matrix is drawn again
  audioMapPath = undefined;
  renderReels();
  renderAssets();
  updateStatusStats();
  submittedPackage = null;
  clearPreviewSelection();
  refreshPreviewCrop();
  refreshDiskSpace();

  await audioMapDrawn;
  let unroutedCells = 0;
  for (const { input, lane, gain } of audioMapCells(form.audioMap)) {
    const cell = document.querySelector(`#prop-audio-map input[data-input="${input}"][data-lane="${lane}"]`);
    if (cell) cell.value = gain;
    else unroutedCells += 1;
  }
  if (unroutedCells) notRestored.push(`${unroutedCells} audio map routes`);

  const joined = [...joinedItems.head, ...joinedItems.tail];
  await refreshLibrary();
  const inLibrary = new Set(libraryItems.map((item) => item.name));
  notRestored.push(...joined.filter((name) => !inLibrary.has(name)).map((name) => `library item ${name}`));
  return notRestored;
}

async function retitleRecentPackage(packagePath, projectPath) {
  const isdcfNaming = getPrefs().isdcfNaming || false;
  const folderName = packagePath.split(/[/\\]/).pop();
  // retitle_dcp rebuilds the ISDCF name around a new title part
  const title = await askForText({
    title: "Retitle DCP",
    label: "New content title",
    value: isdcfNaming ? folderName.split("_")[0] : folderName,
  });
  if (!title?.trim()) return;
  const ok = await tauriConfirm(
    `Retitle to ${title}? The CPL gets a new composition id, so any KDM or delivery made from the old one no longer matches. A signed package loses its signature.`,
    { title: "Retitle DCP", kind: "warning" },
  );
  if (!ok) return;
  let newPath;
  try {
    newPath = await invoke("retitle_dcp", { path: packagePath, title, isdcfNaming });
  } catch (e) {
    tauriMessage(String(e), { title: "Retitle failed", kind: "error" });
    return;
  }
  setStatus(`Retitled to ${title.trim()}`);
  // keeps the project file beside a renamed package folder
  if (newPath === packagePath) addRecentProject(projectPath, title.trim());
  else await moveProjectFile(projectPath, projectPathBeside(newPath), title.trim());
}

async function deleteRecentPackage(packagePath) {
  const ok = await tauriConfirm(`Delete ${packagePath} and everything in it?`, {
    title: "Delete DCP",
    kind: "warning",
  });
  if (!ok) return;
  try {
    await invoke("delete_dcp", { path: packagePath });
  } catch (e) {
    tauriMessage(String(e), { title: "Delete failed", kind: "error" });
    renderRecentProjects();
    return;
  }
  setStatus(`Deleted ${packagePath}`);
  renderRecentProjects();
  refreshDiskSpace();
}

// === Desktop Notifications ===
function notifyBuildComplete(success, title) {
  if (Notification.permission === "granted") {
    new Notification(success ? "Build Complete" : "Build Failed", {
      body: success ? `"${title}" built successfully` : `"${title}" build failed`,
      icon: success ? undefined : undefined,
    });
  } else if (Notification.permission !== "denied") {
    Notification.requestPermission();
  }
}

// Request notification permission early
if ("Notification" in window && Notification.permission === "default") {
  Notification.requestPermission();
}

// === Status Bar Stats ===
function updateStatusStats() {
  const el = document.getElementById("status-stats");
  if (!el) return;
  const n = project.assets.length;
  const v = project.assets.filter(a => a.type === 'video').length;
  const a = project.assets.filter(a => a.type === 'audio').length;
  if (n === 0) { el.textContent = ""; } else {
    const parts = [];
    if (v) parts.push(`${v} video`);
    if (a) parts.push(`${a} audio`);
    const s = project.assets.filter(a => a.type === 'subtitle').length;
    if (s) parts.push(`${s} sub`);
    el.textContent = `${n} assets (${parts.join(', ')})`;
  }
  updateToolbarState();
}

// === Toolbar Button State ===
function beginBuild(title) {
  buildsInFlight.beginBuild(title);
  hidePostBuildActions();
  updateToolbarState();
}

function endBuild(title) {
  buildsInFlight.endBuild(title);
  updateToolbarState();
  refreshDiskSpace();
}

function updateToolbarState() {
  const hasVideo = project.reels.some(r => r.picture);
  const title = document.getElementById("prop-title")?.value?.trim();
  const hasTitle = !!title;
  const buildBtn = document.getElementById("btn-build");
  const previewBtn = document.getElementById("btn-preview");
  if (buildBtn) buildBtn.disabled = !(hasVideo && hasTitle) || buildsInFlight.buildInFlight(title);
  if (previewBtn) previewBtn.disabled = !previewButtonEnabled(previewTarget(previewTargetInput()), previewShownPath);
  refreshMarkerFromPlayerButtons();
}

// Keep title in sync and update toolbar state
const _origTitleHandler = document.getElementById("prop-title");
_origTitleHandler?.addEventListener("input", () => { updateToolbarState(); });

// === Context Menu ===
const ctxMenu = document.getElementById("context-menu");
let ctxAssetId = null;

function showContextMenu(e, assetId) {
  e.preventDefault();
  ctxAssetId = assetId;
  ctxMenu.style.left = e.clientX + "px";
  ctxMenu.style.top = e.clientY + "px";
  ctxMenu.hidden = false;
}

document.addEventListener("click", () => { if (ctxMenu) ctxMenu.hidden = true; });

async function removeAsset(assetId) {
  const asset = project.assets.find(a => a.id === assetId);
  if (!asset) return;
  if (!(await tauriConfirm(`Remove "${asset.name}" from project?`))) return;
  project.assets = project.assets.filter(a => a.id !== assetId);
  if (selectedPreview?.kind === "source" && selectedPreview.path === asset.path) clearPreviewSelection();
  project.reels.forEach(r => {
    if (r.picture?.id === assetId) r.picture = null;
    if (r.sound?.id === assetId) r.sound = null;
    if (r.subtitle?.id === assetId) r.subtitle = null;
  });
  fillFirstReelFromUnplacedAssets();
  renderAssets();
  renderReels();
  updateStatusStats();
}

ctxMenu?.querySelectorAll("button").forEach(btn => {
  btn.addEventListener("click", () => {
    const action = btn.dataset.action;
    const asset = project.assets.find(a => a.id === ctxAssetId);
    if (!asset) return;
    if (action === "preview") {
      previewSourcePicture(asset.path);
    } else if (action === "remove") {
      removeAsset(ctxAssetId);
    } else if (action === "reveal") {
      // reveals the file in the OS file manager (shell open only accepts URLs)
      revealItemInDir(asset.path);
    }
    ctxMenu.hidden = true;
  });
});

// === Progress in Title Bar ===
function setTitleProgress(percent, stage) {
  if (percent === null) {
    setWindowTitleStatus(stage);
  } else if (percent >= 0 && percent < 100) {
    setWindowTitleStatus(`${stage} ${Math.round(percent)}%`);
  } else {
    setWindowTitleStatus("");
  }
}

// === Asset Filter ===
document.getElementById("asset-filter")?.addEventListener("input", (e) => {
  const q = e.target.value.toLowerCase();
  document.querySelectorAll("#asset-list .asset-item").forEach(el => {
    const name = el.querySelector(".asset-name")?.textContent?.toLowerCase() || "";
    el.style.display = name.includes(q) ? "" : "none";
  });
});

// === Auto-detect Video Properties (ffprobe) ===
async function probeVideo(path) {
  try {
    const cmd = Command.create("ffprobe", [
      "-v", "quiet", "-print_format", "json",
      "-show_streams", "-show_format", path
    ]);
    const result = await cmd.execute();
    if (result.code !== 0) return null;
    const info = JSON.parse(result.stdout);
    const vs = info.streams?.find(s => s.codec_type === "video");
    if (!vs) return null;
    return {
      width: vs.width,
      height: vs.height,
      fps: vs.r_frame_rate,
      duration: parseFloat(info.format?.duration || vs.duration || "0"),
    };
  } catch { return null; }
}


// === Ident library ===
// The library itself lives in the app data dir; this holds only what the panel
// draws and which items the next build joins on, in the order they were dropped.
let libraryItems = [];
const joinedItems = { head: [], tail: [] };
const LIBRARY_KIND_LABELS = {
  "head-ident": "Head ident",
  "tail-ident": "Tail ident",
  "rating-card": "Rating card",
  "anti-piracy": "Anti-piracy",
};

async function refreshLibrary() {
  try {
    libraryItems = await invoke("library_list");
  } catch (e) {
    setStatus(`Library: ${e}`);
    libraryItems = [];
  }
  // an item removed from the library cannot stay attached to the next build
  const names = new Set(libraryItems.map(i => i.name));
  for (const placement of ["head", "tail"]) {
    joinedItems[placement] = joinedItems[placement].filter(n => names.has(n));
  }
  renderLibrary();
}

function renderLibrary() {
  const list = document.getElementById("library-list");
  if (!list) return;
  if (libraryItems.length === 0) {
    list.innerHTML = '<div class="asset-empty"><p>No idents yet. Import a clip or a card,<br>then drag it into Head or Tail.</p></div>';
  } else {
    list.innerHTML = libraryItems.map(item => `
      <div class="asset-item" data-library-name="${item.name}" draggable="true">
        <span class="asset-icon">\u{1F39E}</span>
        <span class="asset-name" title="${LIBRARY_KIND_LABELS[item.kind] || item.kind}">${item.name}</span>
        <span class="asset-meta">${item.width}\u00d7${item.height} \u00b7 ${item.seconds.toFixed(1)}s \u00b7 ${item.has_audio ? "sound" : "silent"}</span>
        <button class="asset-remove" data-library-remove="${item.name}" title="Remove from the library">\u2715</button>
      </div>
    `).join("");
    list.querySelectorAll(".asset-item").forEach(el => {
      el.addEventListener("dragstart", (e) => {
        e.dataTransfer.setData("text/plain", libraryPayload(el.dataset.libraryName));
      });
    });
    list.querySelectorAll("[data-library-remove]").forEach(el => {
      el.addEventListener("click", async (e) => {
        e.stopPropagation();
        const name = el.dataset.libraryRemove;
        if (!await tauriConfirm(`Remove "${name}" from the library?`, { title: "Ident library" })) return;
        try {
          await invoke("library_remove", { name });
          setStatus(`Removed ${name} from the library`);
        } catch (err) {
          setStatus(`Library: ${err}`);
        }
        await refreshLibrary();
      });
    });
  }
  renderJoinedItems();
}

function renderJoinedItems() {
  for (const placement of ["head", "tail"]) {
    const box = document.getElementById(`library-${placement}-items`);
    if (!box) continue;
    const names = joinedItems[placement];
    box.innerHTML = names.length === 0
      ? '<div class="library-join-empty">Drop items here</div>'
      : names.map((name, index) => `
        <div class="library-chip" draggable="true" data-placement="${placement}" data-index="${index}">
          <span class="library-chip-name">${index + 1}. ${name}</span>
          <button class="asset-remove" data-detach="${placement}:${index}" title="Take off the build">\u2715</button>
        </div>
      `).join("");
    box.querySelectorAll("[data-detach]").forEach(el => {
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        const [from, index] = el.dataset.detach.split(":");
        joinedItems[from].splice(parseInt(index), 1);
        renderJoinedItems();
      });
    });
    box.querySelectorAll(".library-chip").forEach(chip => {
      chip.addEventListener("dragstart", (e) => {
        e.dataTransfer.setData("text/plain", joinedPayload(chip.dataset.placement, chip.dataset.index));
      });
      // dropping onto a chip puts the dragged item in front of it, which is how
      // a run is reordered
      chip.addEventListener("dragover", (e) => e.preventDefault());
      chip.addEventListener("drop", (e) => {
        e.preventDefault();
        e.stopPropagation();
        dropAndRedraw(e.dataTransfer.getData("text/plain"), placement, parseInt(chip.dataset.index));
      });
    });
  }
}

function dropAndRedraw(payload, placement, at) {
  dropIntoJoin(joinedItems, payload, placement, at);
  renderJoinedItems();
}

document.querySelectorAll(".library-join-items").forEach(box => {
  box.addEventListener("dragover", (e) => {
    e.preventDefault();
    box.classList.add("drop-target");
  });
  box.addEventListener("dragleave", () => box.classList.remove("drop-target"));
  box.addEventListener("drop", (e) => {
    e.preventDefault();
    box.classList.remove("drop-target");
    dropAndRedraw(e.dataTransfer.getData("text/plain"), box.dataset.placement, null);
  });
});

document.getElementById("library-import")?.addEventListener("click", async () => {
  const file = await open({
    filters: [{ name: "Video or image", extensions: ["mov", "mp4", "mkv", "mxf", "avi", "png", "jpg", "jpeg", "tif", "tiff"] }],
  });
  if (!file) return;
  const kind = document.getElementById("library-kind")?.value || "head-ident";
  const hold = parseFloat(document.getElementById("library-hold")?.value);
  const still = await invoke("library_needs_duration", { file });
  if (still && !(hold > 0)) {
    setStatus("A still image has no length of its own: set Hold s before importing it");
    return;
  }
  const name = libraryNameFor(file);
  try {
    const row = await invoke("library_add", { file, name, kind, durationSeconds: still ? hold : null });
    setStatus(`Added ${row.name} to the library`);
  } catch (e) {
    setStatus(`Library: ${e}`);
  }
  await refreshLibrary();
});

// The library names an item by its file, minus the folder, the extension and
// anything a name is not allowed to carry.
function libraryNameFor(path) {
  return path
    .replace(/^.*[/\\]/, "")
    .replace(/\.[^.]*$/, "")
    .replace(/[^A-Za-z0-9 ._-]/g, "-")
    .replace(/^\.+/, "");
}

// === Init ===
renderAssets();
renderReels();
const buildPanelDefaults = serializeBuildPanel();
initProjects({
  wizard: "dcpwizard",
  projectFileVersion: PROJECT_FILE_VERSION,
  projectFileMigrations: PROJECT_FILE_MIGRATIONS,
  applicationName: "DCP Wizard",
  packageNoun: "DCP",
  outputFields: OUTPUT_FIELDS,
  textFields: TEXT_FIELDS,
  defaults: buildPanelDefaults,
  defaultProjectFolder: defaultOutputFolder,
  setProjectTitle,
  setOutputFolder,
  serialize: serializeBuildPanel,
  restore: restoreBuildPanel,
  projectTitle: (form) => form.title?.trim(),
  onQueue: addToPlaylist,
  onRetitle: retitleRecentPackage,
  onDelete: deleteRecentPackage,
  afterRecentRender: applyPreviewSelection,
  setStatus,
});
refreshLibrary();
updateStatusStats();
initPreview();
watchPreviewShown(path => { previewShownPath = path; updateToolbarState(); });
initTimeline();
initPlaylist(document.getElementById("playlist"), { loadPackage: previewPackage });
setStatus("Ready");

// === SRT → SMPTE Subtitle Conversion ===
document.getElementById("srt-browse-input")?.addEventListener("click", async () => {
  const path = await open({ filters: [{ name: "SRT", extensions: ["srt"] }] });
  if (path) {
    document.getElementById("srt-input").value = path;
    document.getElementById("srt-convert").disabled = false;
  }
});
document.getElementById("srt-browse-output")?.addEventListener("click", async () => {
  const path = await open({ directory: true });
  if (path) document.getElementById("srt-output").value = path;
});
document.getElementById("srt-convert")?.addEventListener("click", async () => {
  const input = document.getElementById("srt-input").value;
  const output = document.getElementById("srt-output").value;
  const lang = document.getElementById("srt-language").value || "en";
  const fps = document.getElementById("srt-framerate").value || String(DEFAULT_FRAMERATE);
  const vposition = document.getElementById("srt-vposition").value || "8";
  if (!input) return;

  const resultsEl = document.getElementById("srt-results");
  resultsEl.textContent = "Converting…";
  resultsEl.classList.add("visible");

  try {
    const args = ["subtitle-convert", "-i", input, "-l", lang, "--fps", fps, "--vposition", vposition];
    if (output) args.push("-o", output);
    const cmd = Command.sidecar("dcpwizard", args);
    const result = await cmd.execute();
    resultsEl.textContent = result.code === 0
      ? `✓ Conversion complete\n${result.stdout}`
      : `✗ Error:\n${result.stderr || result.stdout}`;
  } catch (e) {
    resultsEl.textContent = `✗ Failed: ${e}`;
  }
});

// === Subtitle Extraction ===
document.getElementById("subextract-browse-dir")?.addEventListener("click", async () => {
  const path = await open({ directory: true });
  if (path) {
    document.getElementById("subextract-input").value = path;
    document.getElementById("subextract-start").disabled = false;
  }
});
document.getElementById("subextract-browse-file")?.addEventListener("click", async () => {
  const path = await open({ filters: [{ name: "Subtitle", extensions: ["xml", "mxf"] }] });
  if (path) {
    document.getElementById("subextract-input").value = path;
    document.getElementById("subextract-start").disabled = false;
  }
});
document.getElementById("subextract-browse-output")?.addEventListener("click", async () => {
  const path = await open({ directory: true });
  if (path) document.getElementById("subextract-output").value = path;
});
document.getElementById("subextract-start")?.addEventListener("click", async () => {
  const input = document.getElementById("subextract-input").value;
  const format = document.getElementById("subextract-format").value;
  let outDir = document.getElementById("subextract-output").value;
  if (!input) return;
  // default the output beside the input when none is chosen
  if (!outDir) outDir = input.replace(/[\\/][^\\/]*$/, "") || input;

  const resultsEl = document.getElementById("subextract-results");
  resultsEl.textContent = "Extracting…";
  resultsEl.classList.add("visible");

  try {
    const output = outDir + "/subtitles." + format;
    const args = ["subtitle-extract", "-i", input, "-o", output];
    const cmd = Command.sidecar("dcpwizard", args);
    const result = await cmd.execute();
    resultsEl.textContent = result.code === 0
      ? `✓ Extracted to ${output}\n${result.stdout}`
      : `✗ Error:\n${result.stderr || result.stdout}`;
  } catch (e) {
    resultsEl.textContent = `✗ Failed: ${e}`;
  }
});

// === Subtitle Burn-in ===
document.getElementById("burnin-browse-video")?.addEventListener("click", async () => {
  const path = await open({ filters: [{ name: "Video", extensions: ["mp4", "mkv", "mov", "mxf"] }] });
  if (path) {
    document.getElementById("burnin-video").value = path;
    updateBurninBtn();
  }
});
document.getElementById("burnin-browse-sub")?.addEventListener("click", async () => {
  const path = await open({ filters: [{ name: "Subtitle", extensions: ["srt", "xml", "ttml"] }] });
  if (path) {
    document.getElementById("burnin-sub").value = path;
    updateBurninBtn();
  }
});
document.getElementById("burnin-browse-output")?.addEventListener("click", async () => {
  const path = await open({ directory: true });
  if (path) document.getElementById("burnin-output").value = path;
});
function updateBurninBtn() {
  const v = document.getElementById("burnin-video").value;
  const s = document.getElementById("burnin-sub").value;
  document.getElementById("burnin-start").disabled = !(v && s);
}
document.getElementById("burnin-start")?.addEventListener("click", async () => {
  const video = document.getElementById("burnin-video").value;
  const sub = document.getElementById("burnin-sub").value;
  const output = document.getElementById("burnin-output").value;
  if (!video || !sub) return;

  const resultsEl = document.getElementById("burnin-results");
  resultsEl.textContent = "Burning in subtitles…";
  resultsEl.classList.add("visible");

  try {
    const args = ["burnin", "-i", video, "-s", sub];
    if (output) args.push("-o", output);
    const cmd = Command.sidecar("dcpwizard", args);
    const result = await cmd.execute();
    resultsEl.textContent = result.code === 0
      ? `✓ Burn-in complete\n${result.stdout}`
      : `✗ Error:\n${result.stderr || result.stdout}`;
  } catch (e) {
    resultsEl.textContent = `✗ Failed: ${e}`;
  }
});

// === Target Conversion (Scale/Crop/Letterbox) ===
document.getElementById("convert-browse-input")?.addEventListener("click", async () => {
  const path = await open({ filters: [
    { name: "Video", extensions: ["mp4", "mkv", "mov", "mxf"] },
    { name: "All", extensions: ["*"] }
  ]});
  if (path) {
    document.getElementById("convert-input").value = path;
    document.getElementById("convert-start").disabled = false;
  }
});
document.getElementById("convert-browse-output")?.addEventListener("click", async () => {
  const path = await open({ directory: true });
  if (path) document.getElementById("convert-output").value = path;
});
document.getElementById("convert-start")?.addEventListener("click", async () => {
  const input = document.getElementById("convert-input").value;
  const container = document.getElementById("convert-container").value;
  const method = document.getElementById("convert-method").value;
  const output = document.getElementById("convert-output").value;
  if (!input) return;

  const resultsEl = document.getElementById("convert-results");
  resultsEl.textContent = "Converting…";
  resultsEl.classList.add("visible");

  try {
    const args = ["convert", "-i", input, "-t", container, "-m", method];
    if (output) args.push("-o", output);
    const cmd = Command.sidecar("dcpwizard", args);
    const result = await cmd.execute();
    resultsEl.textContent = result.code === 0
      ? `✓ Conversion complete\n${result.stdout}`
      : `✗ Error:\n${result.stderr || result.stdout}`;
  } catch (e) {
    resultsEl.textContent = `✗ Failed: ${e}`;
  }
});

// === Re-ingest Package (rebuild ASSETMAP/PKL) ===
document.getElementById("ingest-browse-dir")?.addEventListener("click", async () => {
  const path = await open({ directory: true });
  if (path) {
    document.getElementById("ingest-dir").value = path;
    document.getElementById("ingest-start").disabled = false;
  }
});
document.getElementById("ingest-start")?.addEventListener("click", async () => {
  const dir = document.getElementById("ingest-dir").value;
  if (!dir) return;

  const resultsEl = document.getElementById("ingest-results");
  resultsEl.textContent = "Rebuilding ASSETMAP and PKL…";
  resultsEl.classList.add("visible");

  try {
    const cmd = Command.sidecar("dcpwizard", ["ingest-package", dir]);
    const result = await cmd.execute();
    resultsEl.textContent = result.code === 0
      ? `✓ Repackaged\n${result.stdout}`
      : `✗ Error:\n${result.stderr || result.stdout}`;
  } catch (e) {
    resultsEl.textContent = `✗ Failed: ${e}`;
  }
});
