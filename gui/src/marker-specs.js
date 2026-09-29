export function markerSpecs(rows) {
  return rows
    .filter(row => row.position.trim())
    .map(row => `${row.label}=${row.position.trim()}`);
}
