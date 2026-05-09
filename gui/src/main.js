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
