import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Command } from "@tauri-apps/plugin-shell";
import { open as _open } from "@tauri-apps/plugin-dialog";
import { documentDir, join } from "@tauri-apps/api/path";
import { initPreview, previewDcp } from "./preview.js";

// === Browse wrapper (remembers last directory) ===
let lastBrowseDir = null;
async function open(opts = {}) {
  const result = await _open({ ...opts, defaultPath: opts.defaultPath || lastBrowseDir || undefined });
  if (result) {
    lastBrowseDir = opts.directory ? result : result.replace(/[/\\][^/\\]*$/, '');
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
document.getElementById("theme-toggle")?.addEventListener("click", () => {
  document.body.classList.toggle("light");
  const btn = document.getElementById("theme-toggle");
  btn.textContent = document.body.classList.contains("light") ? "☀️" : "🌙";
});

// === Preferences (localStorage) ===
const PREFS_KEY = "dcpwizard-preferences";
const PREFS_VERSION = 3;

const PREF_DEFAULTS = {
  standard: "SMPTE", resolution: "2K", framerate: 24,
  encrypt: false, stereo3d: false, validate: true,
  creator: "", facility: "", bandwidth: 250, gpu: -1,
  signingCert: "", signingKey: "", outputDir: "", naming: "",
  channels: "5.1",
};

function getPrefs() {
  try {
    const stored = JSON.parse(localStorage.getItem(PREFS_KEY)) || {};
    if ((stored._version || 0) < PREFS_VERSION) {
      const migrated = { ...PREF_DEFAULTS, ...stored, _version: PREFS_VERSION };
      savePrefs(migrated);
      return migrated;
    }
    return { ...PREF_DEFAULTS, ...stored };
  } catch { return { ...PREF_DEFAULTS, _version: PREFS_VERSION }; }
}

function savePrefs(prefs) {
  prefs._version = PREFS_VERSION;
  localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
}

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
    "set-gpu": prefs.gpu,
    "set-signing-cert": prefs.signingCert,
    "set-signing-key": prefs.signingKey,
    "set-output-dir": prefs.outputDir,
    "set-naming": prefs.naming,
  };
  for (const [id, val] of Object.entries(map)) {
    const el = document.getElementById(id);
    if (el) el.value = val;
  }
}

document.getElementById("settings-form")?.addEventListener("submit", (e) => {
  e.preventDefault();
  const prefs = {
    standard: document.getElementById("set-standard")?.value,
    resolution: document.getElementById("set-resolution")?.value,
    framerate: parseInt(document.getElementById("set-framerate")?.value) || 24,
    creator: document.getElementById("set-creator")?.value,
    facility: document.getElementById("set-facility")?.value,
    bandwidth: parseInt(document.getElementById("set-bandwidth")?.value) || 250,
    gpu: parseInt(document.getElementById("set-gpu")?.value) ?? -1,
    signingCert: document.getElementById("set-signing-cert")?.value,
    signingKey: document.getElementById("set-signing-key")?.value,
    outputDir: document.getElementById("set-output-dir")?.value,
    naming: document.getElementById("set-naming")?.value,
  };
  savePrefs(prefs);
  setStatus("Settings saved");
});

document.getElementById("set-reset")?.addEventListener("click", () => {
  localStorage.removeItem(PREFS_KEY);
  location.reload();
});

loadSettings();

// === Project State ===
const project = {
  title: "",
  assets: [],  // {id, type: 'video'|'audio'|'subtitle', path, name, meta}
  reels: [{ id: 1, picture: null, sound: null, subtitle: null }],
};

let nextAssetId = 1;

// === Drop overlay ===
const dropOverlay = document.getElementById("drop-overlay");

document.addEventListener("dragover", (e) => {
  e.preventDefault();
  if (dropOverlay) dropOverlay.hidden = false;
});
document.addEventListener("dragleave", (e) => {
  if (e.relatedTarget === null && dropOverlay) dropOverlay.hidden = true;
});
document.addEventListener("drop", (e) => {
  e.preventDefault();
  if (dropOverlay) dropOverlay.hidden = true;
  // Handle dropped files
  const files = e.dataTransfer?.files;
  if (files) {
    for (const f of files) {
      importAssetFromPath(f.path || f.name, guessType(f.name));
    }
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

function importAssetFromPath(path, type) {
  const name = path.split(/[/\\]/).pop();
  const asset = { id: nextAssetId++, type, path, name, meta: '' };
  project.assets.push(asset);

  // Auto-assign to first reel if empty
  const reel = project.reels[0];
  if (type === 'video' && !reel.picture) {
    reel.picture = asset;
  } else if (type === 'audio' && !reel.sound) {
    reel.sound = asset;
  } else if (type === 'subtitle' && !reel.subtitle) {
    reel.subtitle = asset;
  }

  renderAssets();
  renderReels();
  setStatus(`Imported: ${name}`);
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
      <span class="asset-meta">${a.type}</span>
    </div>
  `).join('');

  // Make assets draggable to reel tracks
  list.querySelectorAll('.asset-item').forEach(el => {
    el.addEventListener('dragstart', (e) => {
      e.dataTransfer.setData('text/plain', el.dataset.assetId);
    });
  });
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
}

// Add reel button
document.getElementById("add-reel")?.addEventListener("click", () => {
  const maxId = project.reels.reduce((m, r) => Math.max(m, r.id), 0);
  project.reels.push({ id: maxId + 1, picture: null, sound: null, subtitle: null });
  renderReels();
});

// === Output directory ===
document.getElementById("browse-output")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) {
    document.getElementById("prop-output").value = dir;
  }
});

// === Open existing DCP ===
document.getElementById("btn-open-project")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) {
    // Load as a DCP to preview/verify
    const name = dir.split(/[/\\]/).pop();
    document.getElementById("project-name").textContent = name;
    project.title = name;
    document.getElementById("prop-title").value = name;
    setStatus(`Opened: ${dir}`);
    previewDcp(dir);
  }
});

// === Build DCP ===
let currentJobId = null;
let paused = false;

document.getElementById("btn-build")?.addEventListener("click", async () => {
  const title = document.getElementById("prop-title")?.value?.trim();
  if (!title) { alert("Enter a project title in Properties"); return; }

  const reel = project.reels[0];
  if (!reel?.picture) { alert("Import a video asset first"); return; }

  const video = reel.picture.path;
  const audio = reel.sound?.path || null;
  let output = document.getElementById("prop-output")?.value;
  if (!output) {
    const docs = await documentDir();
    output = await join(docs, title);
    document.getElementById("prop-output").value = output;
  }

  // Show progress
  const progressSection = document.getElementById("progress-section");
  const progressBar = document.getElementById("progress-bar");
  const stageEl = document.getElementById("progress-stage");
  const statsEl = document.getElementById("progress-stats");
  progressSection.style.display = "flex";
  progressBar.value = 0;
  stageEl.textContent = "Queued...";
  statsEl.textContent = "";
  paused = false;

  const unlisten = await listen("pipeline-progress", (event) => {
    const p = event.payload;
    if (currentJobId && p.job_id !== currentJobId) return;

    progressBar.value = p.percent;
    stageEl.textContent = p.stage.charAt(0).toUpperCase() + p.stage.slice(1);

    const elapsed = formatTime(p.elapsed_secs);
    let remaining = "";
    if (p.percent > 0 && p.percent < 100) {
      const eta = (p.elapsed_secs / p.percent) * (100 - p.percent);
      remaining = ` ETA ${formatTime(eta)}`;
    }
    const fpsStr = p.fps > 0 ? ` ${p.fps.toFixed(1)}fps` : "";
    statsEl.textContent = `${elapsed}${fpsStr}${remaining}`;

    if (p.stage === "done") {
      setStatus("Build complete");
      unlisten();
      unlistenVal();
    } else if (p.stage === "error") {
      setStatus("Build failed: " + p.message);
      unlisten();
      unlistenVal();
    }
  });

  const unlistenVal = await listen("validation-result", (event) => {
    const v = event.payload;
    if (currentJobId && v.job_id !== currentJobId) return;
    const validEl = document.getElementById("status-validation");
    if (v.valid) {
      validEl.textContent = "✓ Valid";
      validEl.style.color = "#34d399";
    } else {
      validEl.textContent = `✗ ${(v.errors||[]).length} errors`;
      validEl.style.color = "#ff6b6b";
    }
  });

  try {
    currentJobId = await invoke("submit_job", {
      videoPath: video,
      title,
      outputDir: output,
      audioPath: audio,
      validate: document.getElementById("prop-validate")?.checked || false,
    });
    setStatus("Building DCP...");
  } catch (e) {
    stageEl.textContent = "Failed";
    setStatus("Error: " + e);
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
document.getElementById("btn-preview")?.addEventListener("click", async () => {
  const output = document.getElementById("prop-output")?.value;
  if (output) {
    previewDcp(output);
  }
});

// === Verify ===
document.getElementById("verify-browse")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) {
    document.getElementById("verify-path").textContent = dir;
    document.getElementById("verify-run").disabled = false;
  }
});

document.getElementById("verify-run")?.addEventListener("click", async () => {
  const dir = document.getElementById("verify-path").textContent;
  if (!dir || dir.startsWith("No ")) return;

  const resultsBox = document.getElementById("verify-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Verifying...";

  const args = ["verify", dir];
  if (document.getElementById("verify-strict")?.checked) args.push("--strict");
  if (document.getElementById("verify-mxf")?.checked) args.push("--check-mxf");
  if (!document.getElementById("verify-hashes")?.checked) args.push("--skip-hashes");

  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  if (result.code === 0) {
    resultsBox.textContent = "✓ DCP verification PASSED\n\n" + result.stdout;
    setStatus("Verification passed");
  } else {
    resultsBox.textContent = "✗ Verification failed\n\n" + (result.stderr || result.stdout);
    setStatus("Verification failed");
  }
});

// === Security: Encrypt ===
document.getElementById("crypt-browse-dcp")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) {
    document.getElementById("crypt-dcp").value = dir;
    checkEncryptReady();
  }
});
document.getElementById("crypt-browse-cert")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) {
    document.getElementById("crypt-cert").value = file;
    checkEncryptReady();
  }
});

function checkEncryptReady() {
  const btn = document.getElementById("run-encrypt");
  if (btn) btn.disabled = !(document.getElementById("crypt-dcp")?.value && document.getElementById("crypt-cert")?.value);
}

document.getElementById("run-encrypt")?.addEventListener("click", async () => {
  const dcp = document.getElementById("crypt-dcp").value;
  const cert = document.getElementById("crypt-cert").value;
  const resultsBox = document.getElementById("encrypt-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Encrypting...";
  const cmd = Command.sidecar("dcpwizard", ["encrypt", dcp, "--cert", cert]);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? "✓ Encryption complete\n\n" + result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Security: KDM ===
document.getElementById("kdm-browse-dcp")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) { document.getElementById("kdm-dcp").value = dir; checkKdmReady(); }
});
document.getElementById("kdm-browse-cert")?.addEventListener("click", async () => {
  const file = await open({ directory: false });
  if (file) { document.getElementById("kdm-cert").value = file; checkKdmReady(); }
});

function checkKdmReady() {
  const btn = document.getElementById("run-kdm");
  if (btn) btn.disabled = !(document.getElementById("kdm-dcp")?.value && document.getElementById("kdm-cert")?.value);
}

document.getElementById("run-kdm")?.addEventListener("click", async () => {
  const dcp = document.getElementById("kdm-dcp").value;
  const cert = document.getElementById("kdm-cert").value;
  const from = document.getElementById("kdm-from").value;
  const to = document.getElementById("kdm-to").value;
  const resultsBox = document.getElementById("kdm-results");
  resultsBox.classList.add("visible");
  resultsBox.textContent = "Generating KDM...";
  const args = ["kdm", dcp, "--cert", cert];
  if (from) args.push("--from", from);
  if (to) args.push("--to", to);
  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  resultsBox.textContent = result.code === 0
    ? "✓ KDM generated\n\n" + result.stdout
    : "✗ Failed\n\n" + (result.stderr || result.stdout);
});

// === Jobs ===
let jobsPollInterval = null;

async function refreshJobs() {
  const statusBadge = document.getElementById("jobs-status");
  try {
    const result = await Command.sidecar("dcpwizard", ["batch", "list"]).execute();
    if (result.code !== 0) {
      statusBadge.textContent = "Offline";
      document.getElementById("jobs-tbody").innerHTML =
        '<tr><td colspan="5" style="text-align:center">Daemon not running</td></tr>';
      return;
    }
    statusBadge.textContent = "Online";
    const lines = result.stdout.trim().split("\n");
    const tbody = document.getElementById("jobs-tbody");
    if (lines.length <= 1 || lines[0].startsWith("No jobs")) {
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center">No jobs</td></tr>';
      return;
    }
    const jobLines = lines.slice(2).filter(l => l.trim());
    tbody.innerHTML = jobLines.map(line => {
      const parts = line.trim().split(/\s+/);
      const [id, state, progress, type] = parts;
      return `<tr><td>${id}</td><td>${type}</td><td>${state}</td><td>${progress}</td>
        <td>${state === "running" || state === "queued" ? `<button class="btn-sm btn-cancel" data-job-id="${id}">✕</button>` : ''}</td></tr>`;
    }).join('');
    tbody.querySelectorAll(".btn-cancel").forEach(btn => {
      btn.addEventListener("click", async () => {
        await Command.sidecar("dcpwizard", ["batch", "cancel", btn.dataset.jobId]).execute();
        refreshJobs();
      });
    });
  } catch {
    statusBadge.textContent = "Error";
  }
}

function startJobsPolling() {
  if (!jobsPollInterval) jobsPollInterval = setInterval(refreshJobs, 3000);
}
function stopJobsPolling() {
  if (jobsPollInterval) { clearInterval(jobsPollInterval); jobsPollInterval = null; }
}

document.getElementById("jobs-refresh")?.addEventListener("click", refreshJobs);

// === Status bar ===
function setStatus(text) {
  const el = document.getElementById("status-text");
  if (el) el.textContent = text;
}

function formatTime(secs) {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return m > 0 ? `${m}m${s}s` : `${s}s`;
}

// === Title sync ===
document.getElementById("prop-title")?.addEventListener("input", (e) => {
  const title = e.target.value.trim();
  document.getElementById("project-name").textContent = title || "Untitled Project";
  project.title = title;
});

// === Init ===
renderAssets();
renderReels();
initPreview();
setStatus("Ready");
