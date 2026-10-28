const TYPING_HINT = "Click a cell to route it at 0 dB, or type a gain in dB. A dotted cell routes at 0 dB until you type in its row.";
const AUTO_ROUTE_HINT = "Dotted cells show where each channel goes.";
export const AUTO_ROUTE_PLACEHOLDER = "auto";

// a typed gain anywhere in the row replaces the row's auto route
export function rowAutoRoute(row) {
  return row.typed.length ? null : row.autoRoute;
}

function headerCell(lane, upmixLanes) {
  if (!upmixLanes.includes(lane)) return `<th>${lane}</th>`;
  return `<th class="upmix-lane" title="The upmix fills ${lane}">${lane}</th>`;
}

export function audioMapTable(panel) {
  const disabled = panel.typedGainsOff ? " disabled" : "";
  const header = panel.lanes.map((lane) => headerCell(lane, panel.upmixLanes)).join("");
  const rows = panel.channelNames.map((name, channel) => {
    const autoLane = panel.autoRoutes[channel];
    const cells = panel.lanes.map((lane) => {
      const auto = lane === autoLane ? ` class="auto-routed" placeholder="${AUTO_ROUTE_PLACEHOLDER}"` : "";
      return `<td><input type="text" inputmode="decimal"${auto}${disabled} data-input="${channel + 1}" data-lane="${lane}" title="${name} to ${lane}, gain in dB"></td>`;
    }).join("");
    return `<tr><th title="${name}"><span>${name}</span></th>${cells}</tr>`;
  }).join("");
  return `<table style="--lane-count: ${panel.lanes.length}"><thead><tr>${header}</tr></thead><tbody>${rows}</tbody></table>`;
}

// the build refuses typed gains only once a cell holds one
function typingHint(panel, hasTypedGains) {
  if (!panel.typedGainsOff) return TYPING_HINT;
  return hasTypedGains ? `Typed gains are off. ${panel.typedMapRefusal}.` : panel.typedGainsOff;
}

export function audioMapHint(panel, { hasTypedGains }) {
  const parts = [];
  if (panel.upmixDescription) parts.push(panel.upmixDescription);
  if (panel.autoRoutes.some((lane) => lane !== null)) parts.push(AUTO_ROUTE_HINT);
  parts.push(typingHint(panel, hasTypedGains));
  return parts.join(" ");
}
