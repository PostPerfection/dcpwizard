const DECIBEL_STEPS = 10;

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

// the gain field's next value, or null when the button stays disabled
export function gainToReachTarget(targetSpec, measurement, currentGainDb) {
  const target = parseLoudnessTarget(targetSpec);
  if (!target || !measurement) return null;
  const measured = target.metric === "leqm" ? measurement.leqMDb : measurement.integratedLufs;
  const gain = (Number(currentGainDb) || 0) + target.value - measured;
  // adding zero turns a rounded -0 into 0
  return Math.round(gain * DECIBEL_STEPS) / DECIBEL_STEPS + 0;
}

export function measurementSummary(measurement) {
  return `Integrated ${measurement.integratedLufs.toFixed(1)} LUFS, `
    + `Leq(m) ${measurement.leqMDb.toFixed(1)} dB, `
    + `true peak ${measurement.truePeakDbtp.toFixed(2)} dBTP, `
    + `range ${measurement.rangeLu.toFixed(1)} LU`;
}

export function measurementSteps(measurement) {
  return measurement.steps.length ? measurement.steps.join(", ") : "Source track as is";
}
