// what `dcpwizard verify --progress` and `dcpwizard copy --progress` print to stderr
const PROGRESS_LINE = /^progress (hashes|frames|copy) (\d+) (\d+)\s*$/;
const WHOLE_PERCENT = 100;

const STAGE_LABELS = {
  hashes: "Checking hashes",
  frames: "Scanning frames",
  copy: "Copying and verifying",
};

export function parseSidecarProgress(line) {
  const match = PROGRESS_LINE.exec(line);
  if (!match) return null;
  return { stage: match[1], done: Number(match[2]), total: Number(match[3]) };
}

export function sidecarProgressDisplay({ stage, done, total }) {
  const percent = total > 0 ? Math.floor((done * WHOLE_PERCENT) / total) : WHOLE_PERCENT;
  return { percent, text: `${STAGE_LABELS[stage]} ${percent}%` };
}
