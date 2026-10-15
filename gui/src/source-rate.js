const AUTO_FRAME_RATE = "auto";

// conform_source_to_dcp packages an N*1000/1001 source at N fps frame for frame
export function pulledUpRate(sourceRate) {
  const match = /^(\d+)000\/1001$/.exec(sourceRate ?? "");
  return match ? Number(match[1]) : null;
}

// the rate source_rate_to_dcp maps a source onto
function sourceDcpRate(sourceRate) {
  const pulledUp = pulledUpRate(sourceRate);
  if (pulledUp) return pulledUp;
  const [numerator, denominator] = String(sourceRate ?? "").split("/").map(Number);
  if (!numerator || !denominator) return null;
  return Math.trunc(numerator / denominator);
}

// the edit rate the build packages at, with Auto read from the source the way the build reads it
export function editRateOf(framerate, sourceRate, defaultRate) {
  if (framerate !== AUTO_FRAME_RATE) return parseInt(framerate) || defaultRate;
  return sourceDcpRate(sourceRate) ?? defaultRate;
}
