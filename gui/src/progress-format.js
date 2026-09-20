const SECONDS_PER_MINUTE = 60;

// the panel shows these stages' message on its own line
const FINAL_STAGES = new Set(["done", "error", "cancelled"]);

export function formatTime(seconds) {
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  const remainder = Math.floor(seconds % SECONDS_PER_MINUTE);
  return minutes > 0 ? `${minutes}m${remainder}s` : `${remainder}s`;
}

export function progressDisplay(payload) {
  const percent = typeof payload.percent === "number" ? payload.percent : null;
  const name = payload.stage.charAt(0).toUpperCase() + payload.stage.slice(1);
  const withMessage = payload.message && !FINAL_STAGES.has(payload.stage);
  const stage = withMessage ? `${name}: ${payload.message}` : name;

  const fps = payload.fps > 0 ? payload.fps : 0;
  const remaining = fps > 0 ? (payload.total_frames || 0) - (payload.frame || 0) : 0;
  const parts = [formatTime(payload.elapsed_secs)];
  if (fps > 0) parts.push(`${fps.toFixed(1)}fps`);
  if (remaining > 0) parts.push(`ETA ${formatTime(remaining / fps)}`);

  return { stage, stats: parts.join(" "), percent };
}
