import { invoke } from "@tauri-apps/api/core";
import { Command } from "@tauri-apps/plugin-shell";
import { open } from "@tauri-apps/plugin-dialog";

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
document.getElementById("browse-video")?.addEventListener("click", () => browseFolder("video-path"));
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
  "create-form-btn": () => pathSet("video-path") && pathSet("output-path") && document.getElementById("title")?.value?.trim(),
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

// Disable job submit until daemon status is known
const jobSubmitBtn = document.getElementById("job-submit");
if (jobSubmitBtn) jobSubmitBtn.disabled = true;

// === Create form submission ===
document.getElementById("create-form")?.addEventListener("submit", async (e) => {
  e.preventDefault();
  const title = document.getElementById("title").value;
  const video = document.getElementById("video-path").textContent;
  const audio = document.getElementById("audio-path").textContent;
  const output = document.getElementById("output-path").textContent;
  const standard = document.getElementById("standard").value;
  const encrypt = document.getElementById("encrypt-check").checked;

  const args = ["create", "--title", title, "--video", video, "--output", output, "--standard", standard];
  if (pathSet("audio-path")) args.push("--audio", audio);
  if (encrypt) args.push("--encrypt");

  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  if (result.code === 0) {
    alert("DCP created successfully!");
  } else {
    alert("Error: " + result.stderr);
  }
});

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
  Command.sidecar("dcpwizard", ["daemon"]).spawn();
  await new Promise(r => setTimeout(r, 1500));
  refreshJobs();
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
