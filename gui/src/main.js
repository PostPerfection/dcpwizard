import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Command } from "@tauri-apps/plugin-shell";
import { open as _open } from "@tauri-apps/plugin-dialog";
import { documentDir, join } from "@tauri-apps/api/path";
import { initPreview, previewDcp } from "./preview.js";

// Wrapper that remembers last browse location
let lastBrowseDir = null;
async function open(opts = {}) {
  const result = await _open({ ...opts, defaultPath: opts.defaultPath || lastBrowseDir || undefined });
  if (result) {
    lastBrowseDir = opts.directory ? result : result.replace(/[/\\][^/\\]*$/, '');
  }
  return result;
}

// Drop overlay
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
});

// Tab navigation
document.querySelectorAll(".nav-tabs button[data-page]").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".nav-tabs button").forEach((b) => b.classList.remove("active"));
    document.querySelectorAll(".page").forEach((p) => p.classList.remove("active"));
    btn.classList.add("active");
    document.getElementById(btn.dataset.page)?.classList.add("active");
  });
});

// Theme toggle
const themeToggle = document.getElementById("theme-toggle");
themeToggle?.addEventListener("click", () => {
  document.body.classList.toggle("light");
  themeToggle.textContent = document.body.classList.contains("light") ? "☀️" : "🌙";
});

// === Preferences (localStorage, versioned) ===
const PREFS_KEY = "dcpwizard-preferences";
const PREFS_VERSION = 2; // Bump when adding/removing/renaming pref keys

const PREF_DEFAULTS = {
  standard: "SMPTE",
  resolution: "2K",
  framerate: 24,
  encrypt: false,
  stereo3d: false,
  validate: true,
  creator: "",
  facility: "",
  encoder: "grok",
  bandwidth: 250,
  colourspace: "Rec.709",
  gpu: -1,
  signingCert: "",
  signingKey: "",
  ca: "",
  kdmPattern: "%t_%d",
  kdmValidity: 168,
  channels: "5.1",
  loudness: -24,
  outputDir: "",
  naming: "",
};

function getPrefs() {
  try {
    const stored = JSON.parse(localStorage.getItem(PREFS_KEY)) || {};
    if ((stored._version || 0) < PREFS_VERSION) {
      // Migrate: keep existing values, fill in new defaults
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

// Preference field mappings: pref element id -> { type, key }
const prefFields = [
  { id: "pref-standard", type: "select", key: "standard" },
  { id: "pref-resolution", type: "select", key: "resolution" },
  { id: "pref-framerate", type: "number", key: "framerate" },
  { id: "pref-encrypt", type: "checkbox", key: "encrypt" },
  { id: "pref-stereo3d", type: "checkbox", key: "stereo3d" },
  { id: "pref-validate", type: "checkbox", key: "validate" },
  { id: "pref-creator", type: "text", key: "creator" },
  { id: "pref-facility", type: "text", key: "facility" },
  { id: "pref-encoder", type: "select", key: "encoder" },
  { id: "pref-bandwidth", type: "number", key: "bandwidth" },
  { id: "pref-colourspace", type: "select", key: "colourspace" },
  { id: "pref-gpu", type: "number", key: "gpu" },
  { id: "pref-signing-cert", type: "text", key: "signingCert" },
  { id: "pref-signing-key", type: "text", key: "signingKey" },
  { id: "pref-ca", type: "text", key: "ca" },
  { id: "pref-kdm-pattern", type: "text", key: "kdmPattern" },
  { id: "pref-kdm-validity", type: "number", key: "kdmValidity" },
  { id: "pref-channels", type: "select", key: "channels" },
  { id: "pref-loudness", type: "number", key: "loudness" },
  { id: "pref-output-dir", type: "text", key: "outputDir" },
  { id: "pref-naming", type: "text", key: "naming" },
];

// Load prefs into the preference form and apply defaults to the Create form
function loadPrefs() {
  const prefs = getPrefs();

  // Populate preference form
  for (const f of prefFields) {
    const el = document.getElementById(f.id);
    if (!el || !(f.key in prefs)) continue;
    if (f.type === "checkbox") el.checked = prefs[f.key];
    else el.value = prefs[f.key];
  }

  // Apply to Create form defaults
  const stdMap = { "SMPTE": "smpte", "Interop": "interop" };
  const resMap = { "2K": "2k", "4K": "4k" };
  if (prefs.standard) {
    const stdEl = document.getElementById("standard");
    if (stdEl) stdEl.value = stdMap[prefs.standard] || "smpte";
  }
  if (prefs.resolution) {
    const resEl = document.getElementById("resolution");
    if (resEl) resEl.value = resMap[prefs.resolution] || "2k";
  }
  if (prefs.framerate) {
    const frEl = document.getElementById("frame-rate");
    if (frEl) frEl.value = String(prefs.framerate);
  }
  if ("encrypt" in prefs) {
    const encEl = document.getElementById("encrypt-check");
    if (encEl) encEl.checked = prefs.encrypt;
  }
  if ("stereo3d" in prefs) {
    const s3dEl = document.getElementById("stereo3d-check");
    if (s3dEl) s3dEl.checked = prefs.stereo3d;
  }
  if ("validate" in prefs) {
    const valEl = document.getElementById("validate-check");
    if (valEl) valEl.checked = prefs.validate;
  }
}

// Save preferences from the form
document.getElementById("preferences-form")?.addEventListener("submit", (e) => {
  e.preventDefault();
  const prefs = {};
  for (const f of prefFields) {
    const el = document.getElementById(f.id);
    if (!el) continue;
    if (f.type === "checkbox") prefs[f.key] = el.checked;
    else if (f.type === "number") prefs[f.key] = parseFloat(el.value) || 0;
    else prefs[f.key] = el.value;
  }
  savePrefs(prefs);
  loadPrefs(); // re-apply to Create form
  alert("Preferences saved.");
});

// Reset preferences
document.getElementById("pref-reset")?.addEventListener("click", () => {
  localStorage.removeItem(PREFS_KEY);
  location.reload();
});

// Load on startup
loadPrefs();

// === Helper: check if a path display has been set ===
function pathSet(id) {
  const el = document.getElementById(id);
  if (!el) return false;
  const t = el.textContent;
  return t && !t.startsWith("No ");
}

// === Browse helpers ===
async function browseFolder(displayId) {
  const dir = await open({ directory: true });
  if (dir) {
    document.getElementById(displayId).textContent = dir;
    checkFormReady();
  }
}

async function browseFile(displayId) {
  const file = await open({ directory: false });
  if (file) {
    document.getElementById(displayId).textContent = file;
    checkFormReady();
  }
}

// Create page
document.getElementById("browse-video")?.addEventListener("click", async () => {
  const path = await open({
    directory: false,
    multiple: false,
    filters: [
      { name: 'Video', extensions: ['mp4', 'mkv', 'mov', 'avi', 'mxf', 'webm'] },
      { name: 'All Files', extensions: ['*'] }
    ]
  });
  if (path) {
    document.getElementById("video-path").textContent = path;
    checkFormReady();
  }
});
document.getElementById("browse-video-folder")?.addEventListener("click", async () => {
  const dir = await open({ directory: true });
  if (dir) {
    document.getElementById("video-path").textContent = dir;
    checkFormReady();
  }
});
document.getElementById("browse-audio")?.addEventListener("click", () => browseFile("audio-path"));
document.getElementById("browse-output")?.addEventListener("click", () => browseFolder("output-path"));

// Verify page
document.getElementById("browse-verify")?.addEventListener("click", () => browseFolder("verify-path"));

// Encode page
document.getElementById("enc-browse-input")?.addEventListener("click", () => browseFolder("enc-input-path"));
document.getElementById("enc-browse-output")?.addEventListener("click", () => browseFolder("enc-output-path"));

// Transcode page
document.getElementById("tc-browse-input")?.addEventListener("click", () => browseFile("tc-input-path"));
document.getElementById("tc-browse-output")?.addEventListener("click", () => browseFolder("tc-output-path"));

// Subtitles page
document.getElementById("sub-browse-input")?.addEventListener("click", () => browseFile("sub-input-path"));

// Audio page
document.getElementById("aud-browse-input")?.addEventListener("click", () => browseFile("aud-input-path"));

// Encrypt page
document.getElementById("crypt-browse-dcp")?.addEventListener("click", () => browseFolder("crypt-dcp-path"));
document.getElementById("crypt-browse-cert")?.addEventListener("click", () => browseFile("crypt-cert-path"));

// KDM page
document.getElementById("kdm-browse-dcp")?.addEventListener("click", () => browseFolder("kdm-dcp-path"));
document.getElementById("kdm-browse-cert")?.addEventListener("click", () => browseFile("kdm-cert-path"));
document.getElementById("kdm-browse-output")?.addEventListener("click", () => browseFolder("kdm-output-path"));

// Copy page
document.getElementById("copy-browse-source")?.addEventListener("click", () => browseFolder("copy-source-path"));
document.getElementById("copy-browse-dest")?.addEventListener("click", () => browseFolder("copy-dest-path"));

// Loudness page
document.getElementById("loud-browse-input")?.addEventListener("click", () => browseFile("loud-input-path"));

// Report page
document.getElementById("report-browse-dcp")?.addEventListener("click", () => browseFolder("report-dcp-path"));

// === Form validation — disable action buttons until required fields are filled ===
const formRules = {
  "create-form-btn": () => pathSet("video-path") && document.getElementById("title")?.value?.trim(),
  "run-verify": () => pathSet("verify-path"),
  "run-encode": () => pathSet("enc-input-path") && pathSet("enc-output-path"),
  "run-transcode": () => pathSet("tc-input-path") && pathSet("tc-output-path"),
  "run-encrypt": () => pathSet("crypt-dcp-path") && pathSet("crypt-cert-path"),
  "run-kdm": () => pathSet("kdm-dcp-path") && pathSet("kdm-cert-path") && pathSet("kdm-output-path"),
  "run-copy": () => pathSet("copy-source-path") && pathSet("copy-dest-path"),
  "run-loudness": () => pathSet("loud-input-path"),
  "run-report": () => pathSet("report-dcp-path"),
};

function checkFormReady() {
  for (const [btnId, check] of Object.entries(formRules)) {
    const btn = document.getElementById(btnId);
    if (btn) btn.disabled = !check();
  }
}

document.addEventListener("input", checkFormReady);
setTimeout(checkFormReady, 0);

// Auto-set output directory based on title
const titleInput = document.getElementById("title");
titleInput?.addEventListener("input", async () => {
  const title = titleInput.value.trim();
  if (title) {
    const docs = await documentDir();
    const outputPath = await join(docs, title);
    document.getElementById("output-path").textContent = outputPath;
    checkFormReady();
  }
});

// Disable job submit until daemon status is known
const jobSubmitBtn = document.getElementById("job-submit");
if (jobSubmitBtn) jobSubmitBtn.disabled = true;

// === Create DCP — submits a job to the queue ===
let currentJobId = null;
let paused = false;

document.getElementById("create-form")?.addEventListener("submit", async (e) => {
  e.preventDefault();
  const title = document.getElementById("title").value.trim();
  const video = document.getElementById("video-path").textContent;
  const audio = document.getElementById("audio-path").textContent;
  const output = document.getElementById("output-path").textContent;

  const progressDiv = document.getElementById("pipeline-progress");
  const progressBar = document.getElementById("pipeline-bar");
  const stageEl = document.getElementById("pipeline-stage");
  const statsEl = document.getElementById("pipeline-stats");
  const msgEl = document.getElementById("pipeline-message");
  const validationDiv = document.getElementById("validation-results");

  progressDiv.style.display = "block";
  validationDiv.style.display = "none";
  validationDiv.innerHTML = "";
  progressBar.value = 0;
  stageEl.textContent = "Queued...";
  statsEl.textContent = "";
  msgEl.textContent = "";
  paused = false;
  document.getElementById("pipeline-pause").textContent = "⏸";

  document.getElementById("create-form-btn").disabled = true;

  const unlisten = await listen("pipeline-progress", (event) => {
    const p = event.payload;
    if (currentJobId && p.job_id !== currentJobId) return;

    progressBar.value = p.percent;
    stageEl.textContent = p.stage.charAt(0).toUpperCase() + p.stage.slice(1);
    msgEl.textContent = p.message;

    const elapsed = formatTime(p.elapsed_secs);
    let remaining = "";
    if (p.percent > 0 && p.percent < 100) {
      const eta = (p.elapsed_secs / p.percent) * (100 - p.percent);
      remaining = ` | ETA: ${formatTime(eta)}`;
    }
    const fpsStr = p.fps > 0 ? ` | ${p.fps.toFixed(1)} fps` : "";
    statsEl.textContent = `${elapsed}${fpsStr}${remaining}`;

    if (p.stage === "done") {
      document.getElementById("create-form-btn").disabled = false;
      // Show the output path in the DCP preview field (don't auto-play)
      const output = document.getElementById("output-path").textContent;
      if (output && !output.startsWith("No ")) {
        document.getElementById("prev-dcp-path").textContent = output;
      }
      unlisten();
      unlistenValidation();
    } else if (p.stage === "error") {
      document.getElementById("create-form-btn").disabled = false;
      unlisten();
      unlistenValidation();
    }
  });

  const unlistenValidation = await listen("validation-result", (event) => {
    const v = event.payload;
    if (currentJobId && v.job_id !== currentJobId) return;

    validationDiv.style.display = "block";
    let html = "";
    if (v.valid) {
      validationDiv.style.background = "var(--success-bg, #1a3a1a)";
      html = "<strong>✓ DCP is valid</strong>";
    } else {
      validationDiv.style.background = "var(--error-bg, #3a1a1a)";
      html = "<strong>✗ Validation issues found</strong><br>";
    }
    for (const err of v.errors || []) {
      html += `<div style="color:#ff6b6b;">ERROR: ${err}</div>`;
    }
    for (const warn of v.warnings || []) {
      html += `<div style="color:#ffa500;">WARNING: ${warn}</div>`;
    }
    validationDiv.innerHTML = html;
  });

  try {
    currentJobId = await invoke("submit_job", {
      videoPath: video,
      title: title,
      outputDir: output,
      audioPath: pathSet("audio-path") ? audio : null,
      validate: document.getElementById("validate-check")?.checked || false,
    });
  } catch (e) {
    msgEl.textContent = "Error: " + e;
    stageEl.textContent = "Failed";
    document.getElementById("create-form-btn").disabled = false;
    unlisten();
    unlistenValidation();
  }
});

// Pause / Resume
document.getElementById("pipeline-pause")?.addEventListener("click", async () => {
  if (paused) {
    await invoke("resume_job");
    paused = false;
    document.getElementById("pipeline-pause").textContent = "⏸";
  } else {
    await invoke("pause_job");
    paused = true;
    document.getElementById("pipeline-pause").textContent = "▶";
    document.getElementById("pipeline-stage").textContent += " (paused)";
  }
});

// Cancel
document.getElementById("pipeline-cancel")?.addEventListener("click", async () => {
  if (currentJobId) {
    await invoke("cancel_job", { jobId: currentJobId });
  }
});

function formatTime(secs) {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return m > 0 ? `${m}m ${s}s` : `${s}s`;
}

// === Verify ===
document.getElementById("run-verify")?.addEventListener("click", async () => {
  const dir = document.getElementById("verify-path").textContent;
  const cmd = Command.sidecar("dcpwizard", ["verify", dir]);
  const result = await cmd.execute();
  const box = document.getElementById("verify-results");
  if (result.code === 0) {
    box.innerHTML = '<p class="pass">✅ DCP verification PASSED</p>';
  } else {
    box.innerHTML = `<p class="fail">❌ Verification failed</p><pre>${result.stderr}</pre>`;
  }
});

// === Jobs page ===
let jobsPollInterval = null;

async function refreshJobs() {
  const statusBadge = document.getElementById("jobs-daemon-status");
  const submitBtn = document.getElementById("job-submit");
  try {
    const ping = Command.sidecar("dcpwizard", ["batch", "list"]);
    const result = await ping.execute();
    if (result.code !== 0) {
      statusBadge.textContent = "Daemon offline";
      statusBadge.className = "status-badge offline";
      if (submitBtn) submitBtn.disabled = true;
      document.getElementById("jobs-tbody").innerHTML =
        '<tr><td colspan="6" style="text-align:center">Daemon not running</td></tr>';
      return;
    }
    statusBadge.textContent = "Daemon online";
    statusBadge.className = "status-badge online";
    if (submitBtn) submitBtn.disabled = false;

    const lines = result.stdout.trim().split("\n");
    const tbody = document.getElementById("jobs-tbody");

    if (lines.length <= 1 || lines[0].startsWith("No jobs")) {
      tbody.innerHTML = '<tr><td colspan="6" style="text-align:center">No jobs in queue</td></tr>';
      return;
    }

    const jobLines = lines.slice(2).filter(l => l.trim());
    tbody.innerHTML = jobLines.map(line => {
      const parts = line.trim().split(/\s+/);
      const id = parts[0];
      const state = parts[1];
      const progress = parts[2];
      const type = parts[3];
      const desc = parts.slice(4).join(" ");
      const cancelBtn = (state === "queued" || state === "running")
        ? `<button class="btn-cancel" data-job-id="${id}">Cancel</button>`
        : "";
      const stateClass = `state-${state}`;
      return `<tr>
        <td>${id}</td>
        <td>${type}</td>
        <td>${desc}</td>
        <td><span class="${stateClass}">${state}</span></td>
        <td>${progress}</td>
        <td>${cancelBtn}</td>
      </tr>`;
    }).join("");

    tbody.querySelectorAll(".btn-cancel").forEach(btn => {
      btn.addEventListener("click", async () => {
        const jobId = btn.dataset.jobId;
        await Command.sidecar("dcpwizard", ["batch", "cancel", jobId]).execute();
        refreshJobs();
      });
    });
  } catch (err) {
    statusBadge.textContent = "Error";
    statusBadge.className = "status-badge offline";
    if (submitBtn) submitBtn.disabled = true;
    document.getElementById("jobs-tbody").innerHTML =
      `<tr><td colspan="6" style="text-align:center;color:var(--accent)">${err}</td></tr>`;
  }
}

document.getElementById("jobs-refresh")?.addEventListener("click", refreshJobs);

document.getElementById("jobs-start-daemon")?.addEventListener("click", async () => {
  const statusBadge = document.getElementById("jobs-daemon-status");
  statusBadge.textContent = "Starting...";
  statusBadge.className = "status-badge";
  try {
    const child = await Command.sidecar("dcpwizard", ["daemon"]).spawn();
    console.log("Daemon spawned, pid:", child.pid);
  } catch (err) {
    console.error("Failed to spawn daemon:", err);
    statusBadge.textContent = "Failed to start";
    statusBadge.className = "status-badge offline";
    return;
  }
  // Poll until daemon is reachable (up to 5 seconds)
  for (let i = 0; i < 10; i++) {
    await new Promise(r => setTimeout(r, 500));
    try {
      const result = await Command.sidecar("dcpwizard", ["batch", "list"]).execute();
      if (result.code === 0) {
        refreshJobs();
        return;
      }
    } catch (_) {}
  }
  statusBadge.textContent = "Failed to start";
  statusBadge.className = "status-badge offline";
});

document.getElementById("job-submit")?.addEventListener("click", async () => {
  const type = document.getElementById("job-type").value;
  const paramsStr = document.getElementById("job-args").value;
  if (!paramsStr) { alert("Enter job parameters (JSON)"); return; }

  const args = ["batch", "add", "-T", type, "-p", paramsStr];
  const result = await Command.sidecar("dcpwizard", args).execute();
  if (result.code === 0) {
    document.getElementById("job-args").value = "";
    refreshJobs();
  } else {
    alert("Failed to submit: " + result.stderr);
  }
});

// Auto-refresh jobs when tab is active
document.querySelectorAll(".nav-tabs button[data-page]").forEach(btn => {
  btn.addEventListener("click", () => {
    if (btn.dataset.page === "jobs-page") {
      refreshJobs();
      if (!jobsPollInterval) {
        jobsPollInterval = setInterval(refreshJobs, 3000);
      }
    } else {
      if (jobsPollInterval) { clearInterval(jobsPollInterval); jobsPollInterval = null; }
    }
  });
});

// Initialize preview player
initPreview();
