export function filledRatings(rows) {
  return rows
    .filter((row) => row.agency.trim() && row.label.trim())
    .map((row) => ({ agency: row.agency.trim(), label: row.label.trim() }));
}

export function ratingRowsFrom(ratings) {
  return ratings.map(({ agency, label }, index) => ({ id: index + 1, agency, label }));
}

export function nextRatingRowId(rows) {
  return Math.max(0, ...rows.map((row) => row.id)) + 1;
}
