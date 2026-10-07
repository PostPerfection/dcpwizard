const SMALLEST_CHANNEL_SET = 2;

export function isChannelSet(asset) {
  return Array.isArray(asset?.channelFiles);
}

export function channelSetMeta(files) {
  return `${files.length} ch: ${files.map((file) => file.lane).join(" ")}`;
}

export function channelSetPreviewPath(asset) {
  return isChannelSet(asset) ? null : asset.path;
}

export function soundSource(asset) {
  if (!asset) return { audioPath: null, audioChannelFiles: null };
  if (isChannelSet(asset)) return { audioPath: null, audioChannelFiles: asset.channelFiles.map((file) => file.path) };
  return { audioPath: asset.path, audioChannelFiles: null };
}

function soundFilePaths(asset) {
  if (asset.type !== "audio") return [];
  return isChannelSet(asset) ? asset.channelFiles.map((file) => file.path) : [asset.path];
}

// groups come from group_channel_files over the paths of the audio assets
export function mergeChannelSets(assets, groups, takeAssetId) {
  let merged = [...assets];
  const absorbedInto = new Map();
  for (const group of groups) {
    if (group.files.length < SMALLEST_CHANNEL_SET) continue;
    const groupPaths = new Set(group.files.map((file) => file.path));
    const absorbed = merged.filter((asset) => {
      const paths = soundFilePaths(asset);
      return paths.length > 0 && paths.every((path) => groupPaths.has(path));
    });
    const alreadyThisSet = absorbed.length === 1 && isChannelSet(absorbed[0]) && absorbed[0].channelFiles.length === groupPaths.size;
    if (alreadyThisSet) continue;
    const files = group.files.map(({ path, lane }) => ({ path, lane }));
    const set = { id: takeAssetId(), type: "audio", name: group.prefix, meta: channelSetMeta(files), path: null, channelFiles: files };
    const position = merged.findIndex((asset) => absorbed.includes(asset));
    merged = merged.flatMap((asset, index) => {
      if (index === position) return [set];
      return absorbed.includes(asset) ? [] : [asset];
    });
    if (position === -1) merged.push(set);
    for (const asset of absorbed) absorbedInto.set(asset.id, set);
  }
  return { assets: merged, absorbedInto };
}
