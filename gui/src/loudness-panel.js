import { loudnessChartSvg } from "./loudness-chart.js";

const DECIBEL_STEPS = 10;
const STALE_NOTE = ". Measure again to confirm";

// the same spellings the build's loudness target accepts, null when it would refuse
export function parseLoudnessTarget(spec) {
  const text = String(spec ?? "");
  const separator = text.indexOf("=");
  if (separator < 0) return null;
  const metric = text.slice(0, separator).trim().toLowerCase().replace("(m)", "");
  const valueText = text.slice(separator + 1).trim();
  const value = Number(valueText);
  if (valueText === "" || !Number.isFinite(value)) return null;
  if (metric === "leqm" || metric === "leq") return { metric: "leqm", value };
  if (metric === "lufs") return { metric: "lufs", value };
  return null;
}

// the gain that takes the sound before its level step to the target, or null to keep the button disabled
export function gainToReachTarget(targetSpec, measurement) {
  const target = parseLoudnessTarget(targetSpec);
  if (!target || !measurement) return null;
  const before = target.metric === "leqm" ? measurement.before.leqMDb : measurement.before.integratedLufs;
  // adding zero turns a rounded -0 into 0
  return Math.round((target.value - before) * DECIBEL_STEPS) / DECIBEL_STEPS + 0;
}

// a typed 0 is a gain, so only an empty field is no gain
export function optionalNumber(text) {
  const trimmed = String(text ?? "").trim();
  if (trimmed === "") return null;
  const value = Number(trimmed);
  return Number.isFinite(value) ? value : null;
}

function levelsSummary(levels) {
  return `Integrated ${levels.integratedLufs.toFixed(1)} LUFS, `
    + `Leq(m) ${levels.leqMDb.toFixed(1)} dB, `
    + `true peak ${levels.truePeakDbtp.toFixed(2)} dBTP, `
    + `range ${levels.rangeLu.toFixed(1)} LU`;
}

export function sourceLine(measurement) {
  return `Source: ${levelsSummary(measurement.before)}`;
}

export function deliveredLine(measurement, stale = false) {
  return `Delivered: ${levelsSummary(measurement.delivered)}${stale ? STALE_NOTE : ""}`;
}

export function measurementSteps(measurement) {
  return measurement.steps.length ? measurement.steps.join(", ") : "Source track as is";
}

export function deliveredChart(measurement, targetSpec) {
  return loudnessChartSvg(measurement.delivered, parseLoudnessTarget(targetSpec));
}
