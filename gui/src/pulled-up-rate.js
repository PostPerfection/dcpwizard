// conform_source_to_dcp packages an N*1000/1001 source at N fps frame for frame
export function pulledUpRate(sourceRate) {
  const match = /^(\d+)000\/1001$/.exec(sourceRate ?? "");
  return match ? Number(match[1]) : null;
}
