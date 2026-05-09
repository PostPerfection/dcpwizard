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

// Browse buttons
async function browseFolder(displayId) {
  const dir = await open({ directory: true });
  if (dir) document.getElementById(displayId).textContent = dir;
}

async function browseFile(displayId) {
  const file = await open({ directory: false });
  if (file) document.getElementById(displayId).textContent = file;
}

document.getElementById("browse-video")?.addEventListener("click", () => browseFolder("video-path"));
document.getElementById("browse-audio")?.addEventListener("click", () => browseFile("audio-path"));
document.getElementById("browse-output")?.addEventListener("click", () => browseFolder("output-path"));
document.getElementById("browse-verify")?.addEventListener("click", async () => {
  await browseFolder("verify-path");
  document.getElementById("run-verify").disabled = false;
});

// Create form submission
document.getElementById("create-form")?.addEventListener("submit", async (e) => {
  e.preventDefault();
  const title = document.getElementById("title").value;
  const video = document.getElementById("video-path").textContent;
  const audio = document.getElementById("audio-path").textContent;
  const output = document.getElementById("output-path").textContent;
  const standard = document.getElementById("standard").value;
  const encrypt = document.getElementById("encrypt-check").checked;

  if (!title || video === "No folder selected" || output === "No folder selected") {
    alert("Please fill in title, video source, and output directory.");
    return;
  }

  const args = ["create", "--title", title, "--video", video, "--output", output, "--standard", standard];
  if (audio !== "No file selected") args.push("--audio", audio);
  if (encrypt) args.push("--encrypt");

  const cmd = Command.sidecar("dcpwizard", args);
  const result = await cmd.execute();
  if (result.code === 0) {
    alert("DCP created successfully!");
  } else {
    alert("Error: " + result.stderr);
  }
});

// Verify
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
