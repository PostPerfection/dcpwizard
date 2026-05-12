// Preview player - uses mpv via IPC for high-performance video playback
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

let lastBrowsePath = null;

export function initPreview() {
  // Preview source video (button next to browse-video)
  const previewSourceBtn = document.getElementById('preview-source');
  previewSourceBtn?.addEventListener('click', () => {
    const path = document.getElementById('video-path')?.textContent;
    if (path && !path.startsWith('No ')) {
      invoke('preview_load', { filePath: path }).catch((e) => {
        console.error('[preview] Failed to load source:', e);
      });
    }
  });

  // DCP replay
  const replayDcpBtn = document.getElementById('prev-replay-dcp');
  replayDcpBtn?.addEventListener('click', async () => {
    const path = await open({ directory: true, multiple: false, defaultPath: lastBrowsePath || undefined });
    if (path) {
      lastBrowsePath = path;
      document.getElementById('prev-dcp-path').textContent = path;
      invoke('preview_load_dcp', { dirPath: path }).catch((e) => {
        console.error('[preview] Failed to load DCP:', e);
        document.getElementById('prev-time').textContent = 'Error: ' + e;
      });
    }
  });

  // Keyboard shortcuts
  document.addEventListener('keydown', (e) => {
    if (document.getElementById('create-page')?.classList.contains('active')) {
      if (e.key === ' ' && e.target.tagName !== 'INPUT' && e.target.tagName !== 'SELECT') {
        e.preventDefault(); invoke('preview_play_pause');
      }
      if (e.key === 'ArrowLeft') invoke('preview_seek', { seconds: -5.0 });
      if (e.key === 'ArrowRight') invoke('preview_seek', { seconds: 5.0 });
    }
  });
}

/// Load a DCP into the preview player (called from outside after job completes)
export function previewDcp(dirPath) {
  document.getElementById('prev-dcp-path').textContent = dirPath;
  invoke('preview_load_dcp', { dirPath }).catch((e) => {
    console.error('[preview] Failed to auto-load DCP:', e);
  });
}
