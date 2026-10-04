const LIBRARY_PAYLOAD_PREFIX = "library:";
const JOINED_PAYLOAD_PREFIX = "joined:";

export function libraryPayload(name) {
  return `${LIBRARY_PAYLOAD_PREFIX}${name}`;
}

export function joinedPayload(placement, index) {
  return `${JOINED_PAYLOAD_PREFIX}${placement}:${index}`;
}

export function dropIntoJoin(joinedItems, payload, placement, at) {
  const target = joinedItems[placement];
  let name = null;
  if (payload.startsWith(LIBRARY_PAYLOAD_PREFIX)) {
    name = payload.slice(LIBRARY_PAYLOAD_PREFIX.length);
  } else if (payload.startsWith(JOINED_PAYLOAD_PREFIX)) {
    const [, from, index] = payload.split(":");
    const removed = joinedItems[from].splice(parseInt(index), 1);
    if (removed.length === 0) return;
    name = removed[0];
    // taking it out of this same run shifts everything after it back one
    if (from === placement && at !== null && at > parseInt(index)) at -= 1;
  }
  if (!name) return;
  if (at === null || at > target.length) target.push(name);
  else target.splice(at, 0, name);
}
