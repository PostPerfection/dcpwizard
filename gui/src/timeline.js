// DCP Timeline View - renders multi-reel timeline with visual reel segments and playback integration
import { invoke } from '@tauri-apps/api/core';
import { projectTimelineEntries, reelPositionAtFrame, segmentSpan, timelineFrameOfReelPosition } from './project-timeline.js';

let timelineData = null; // { reels: [], totalFrames, editRate }
let currentReel = -1;
let playheadFrame = 0;
let timelinePollingId = null;
// an opened DCP plays as one composition, a project's preview plays one reel's picture at a time
let followsProjectReels = false;
// the reel whose picture the preview plays, null while it plays anything else
let previewedReelIndex = null;
let seekProjectReel = async () => {};

export function initTimeline() {
  renderEmpty();
}

// Load timeline from an opened DCP's CPL
export async function loadTimelineFromCpl(cplPath) {
  try {
    const reels = await invoke('get_timeline', { cplPath });
    if (!reels || reels.length === 0) {
      renderEmpty();
      return;
    }
    buildTimelineData(reels);
    followsProjectReels = false;
    render();
    startTimelinePolling();
  } catch (e) {
    console.error('[timeline] Failed to load CPL:', e);
    renderEmpty();
  }
}

// Load timeline from the project model (for DCPs being built)
export function loadTimelineFromProject(reels, durationsFrames, editRate) {
  if (!reels || reels.length === 0) {
    renderEmpty();
    return;
  }
  buildTimelineData(projectTimelineEntries(reels, durationsFrames, editRate));
  followsProjectReels = true;
  render();
  startTimelinePolling();
}

export function setPreviewedTimelineReel(reelIndex) {
  previewedReelIndex = reelIndex;
}

// the page loads a reel's picture into the preview and seeks it
export function setProjectReelSeeker(seeker) {
  seekProjectReel = seeker;
}

function buildTimelineData(reels) {
  let totalFrames = 0;
  const parsed = reels.map(r => {
    const fps = parseEditRate(r.edit_rate);
    const startFrame = totalFrames;
    totalFrames += r.duration_frames;
    return { ...r, startFrame, fps };
  });
  timelineData = { reels: parsed, totalFrames, editRate: parsed[0]?.fps || 24 };
}

function parseEditRate(er) {
  if (!er) return 24;
  const parts = er.trim().split(/\s+/);
  if (parts.length === 2) return Math.round(parseInt(parts[0]) / parseInt(parts[1]));
  return parseInt(parts[0]) || 24;
}

function renderEmpty() {
  const ruler = document.getElementById('timeline-ruler');
  const picture = document.getElementById('timeline-picture');
  const sound = document.getElementById('timeline-sound');
  const subtitle = document.getElementById('timeline-subtitle');
  if (ruler) ruler.innerHTML = '<span class="timeline-empty-msg">Open a DCP or build a project to see the timeline</span>';
  if (picture) picture.innerHTML = '';
  if (sound) sound.innerHTML = '';
  if (subtitle) subtitle.innerHTML = '';
  timelineData = null;
  currentReel = -1;
}

function render() {
  if (!timelineData || timelineData.reels.length === 0) {
    renderEmpty();
    return;
  }
  renderRuler();
  renderTracks();
  updatePlayheadPosition();
}

function renderRuler() {
  const ruler = document.getElementById('timeline-ruler');
  if (!ruler) return;
  ruler.innerHTML = '';
  ruler.style.position = 'relative';

  const { totalFrames, editRate } = timelineData;
  if (totalFrames === 0) return;

  // Generate tick marks at sensible intervals
  const totalSeconds = totalFrames / editRate;
  const interval = getTickInterval(totalSeconds);

  for (let t = 0; t <= totalSeconds; t += interval) {
    const pct = (t / totalSeconds) * 100;
    const tick = document.createElement('div');
    tick.className = 'ruler-tick';
    tick.style.left = `${pct}%`;
    tick.dataset.time = formatTimecodeShort(t, editRate);

    const label = document.createElement('span');
    label.className = 'ruler-label';
    label.textContent = formatTimecodeShort(t, editRate);
    tick.appendChild(label);

    ruler.appendChild(tick);
  }

  // Reel boundary markers
  for (const reel of timelineData.reels) {
    if (reel.startFrame > 0) {
      const pct = (reel.startFrame / totalFrames) * 100;
      const marker = document.createElement('div');
      marker.className = 'ruler-reel-marker';
      marker.style.left = `${pct}%`;
      marker.title = `Reel ${reel.reel_number}`;
      ruler.appendChild(marker);
    }
  }

  // Playhead in ruler
  const playhead = document.createElement('div');
  playhead.className = 'ruler-playhead';
  playhead.id = 'ruler-playhead';
  ruler.appendChild(playhead);

  // Click on ruler to seek
  ruler.onmousedown = handleRulerSeek;
}

function renderTracks() {
  const pictureEl = document.getElementById('timeline-picture');
  const soundEl = document.getElementById('timeline-sound');
  const subtitleEl = document.getElementById('timeline-subtitle');
  if (!pictureEl || !soundEl) return;

  pictureEl.innerHTML = '';
  soundEl.innerHTML = '';
  if (subtitleEl) subtitleEl.innerHTML = '';

  const { reels, totalFrames } = timelineData;
  const colors = ['#7c3aed', '#6d28d9', '#5b21b6', '#4c1d95', '#8b5cf6'];
  const soundColors = ['#3b82f6', '#2563eb', '#1d4ed8', '#1e40af', '#60a5fa'];
  const subtitleColors = ['#d97706', '#b45309', '#92400e', '#78350f', '#f59e0b'];

  for (const reel of reels) {
    const { leftPercent: leftPct, widthPercent: widthPct } = segmentSpan(reel, reels.length, totalFrames);

    // Picture segment
    if (reel.picture_file || reel.picture_asset_id) {
      const seg = createSegment(reel, leftPct, widthPct, colors[(reel.reel_number - 1) % colors.length], 'picture');
      pictureEl.appendChild(seg);
    }

    // Sound segment
    if (reel.sound_file || reel.sound_asset_id) {
      const seg = createSegment(reel, leftPct, widthPct, soundColors[(reel.reel_number - 1) % soundColors.length], 'sound');
      soundEl.appendChild(seg);
    }

    if (subtitleEl && (reel.subtitle_file || reel.subtitle_asset_id)) {
      const seg = createSegment(reel, leftPct, widthPct, subtitleColors[(reel.reel_number - 1) % subtitleColors.length], 'subtitle');
      subtitleEl.appendChild(seg);
    }
  }

  // an added listener would pile up on every render
  pictureEl.onmousedown = handleTrackSeek;
  soundEl.onmousedown = handleTrackSeek;
  if (subtitleEl) subtitleEl.onmousedown = handleTrackSeek;
}

function createSegment(reel, leftPct, widthPct, color, type) {
  const seg = document.createElement('div');
  seg.className = 'timeline-segment' + (reel.reel_number - 1 === currentReel ? ' active' : '');
  seg.style.left = `${leftPct}%`;
  seg.style.width = `${widthPct}%`;
  seg.style.backgroundColor = color;
  seg.dataset.reelIndex = reel.reel_number - 1;
  seg.dataset.type = type;

  const label = document.createElement('span');
  label.className = 'segment-label';
  label.textContent = `R${reel.reel_number}`;
  seg.appendChild(label);

  const dur = document.createElement('span');
  dur.className = 'segment-duration';
  dur.textContent = formatTimecodeShort(reel.duration_frames / (reel.fps || 24), reel.fps || 24);
  seg.appendChild(dur);

  return seg;
}

function handleRulerSeek(e) {
  const ruler = document.getElementById('timeline-ruler');
  if (!ruler || !timelineData) return;
  const rect = ruler.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  seekToPercent(pct);
}

function handleTrackSeek(e) {
  const track = e.currentTarget;
  if (!track || !timelineData) return;
  const rect = track.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  seekToPercent(pct);
}

// the player holds the whole composition, so a seek is in composition seconds
async function seekToPercent(pct) {
  if (!timelineData) return;
  const targetFrame = Math.floor(pct * timelineData.totalFrames);
  playheadFrame = targetFrame;
  updatePlayheadPosition();

  try {
    if (!followsProjectReels) {
      await invoke('preview_seek_absolute', { seconds: targetFrame / (timelineData.editRate || 24) });
    } else if (previewedReelIndex !== null) {
      const { reelIndex, seconds } = reelPositionAtFrame(timelineData.reels, targetFrame);
      await seekProjectReel(reelIndex, seconds);
    }
  } catch (e) {
    console.error('[timeline] Failed to seek:', e);
  }
}

function reelAtFrame(frame) {
  const index = timelineData.reels.findIndex(
    reel => frame >= reel.startFrame && frame < reel.startFrame + reel.duration_frames,
  );
  return index === -1 ? timelineData.reels.length - 1 : index;
}

function updatePlayheadPosition() {
  if (!timelineData || timelineData.totalFrames === 0) return;
  currentReel = reelAtFrame(playheadFrame);
  const pct = (playheadFrame / timelineData.totalFrames) * 100;

  const rulerPlayhead = document.getElementById('ruler-playhead');
  if (rulerPlayhead) rulerPlayhead.style.left = `${pct}%`;

  // Update segment active states
  document.querySelectorAll('.timeline-segment').forEach(seg => {
    const idx = parseInt(seg.dataset.reelIndex);
    seg.classList.toggle('active', idx === currentReel);
  });
}

export function startTimelinePolling() {
  if (timelinePollingId) return;
  timelinePollingId = setInterval(async () => {
    if (!timelineData) return;
    try {
      const resp = await invoke('preview_get_metadata');
      const meta = JSON.parse(resp);
      if (meta.position == null) return;
      if (!followsProjectReels) {
        // the position is where playback sits in the composition, not in a reel
        const fps = timelineData.editRate || 24;
        playheadFrame = Math.min(Math.floor(meta.position * fps), timelineData.totalFrames);
      } else {
        const reel = timelineData.reels[previewedReelIndex];
        if (!reel) return;
        playheadFrame = timelineFrameOfReelPosition(reel, meta.position);
      }
      updatePlayheadPosition();
    } catch {
      // mpv not running
    }
  }, 250);
}

export function stopTimelinePolling() {
  if (timelinePollingId) {
    clearInterval(timelinePollingId);
    timelinePollingId = null;
  }
}

// Utility: get a sensible tick interval based on total duration
function getTickInterval(totalSeconds) {
  if (totalSeconds <= 10) return 1;
  if (totalSeconds <= 30) return 5;
  if (totalSeconds <= 60) return 10;
  if (totalSeconds <= 300) return 30;
  if (totalSeconds <= 600) return 60;
  if (totalSeconds <= 1800) return 300;
  return 600;
}

function formatTimecodeShort(seconds, fps) {
  if (!seconds || seconds <= 0) return '0:00';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return `${m}:${String(s).padStart(2, '0')}`;
}
