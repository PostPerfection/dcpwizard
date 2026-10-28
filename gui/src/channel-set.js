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

// files named only by their channel, such as FL.wav, share the empty prefix
function channelSetName(group) {
  return group.prefix || group.files[0].path.split(/[/\\]/).at(-2);
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
    const set = { id: takeAssetId(), type: "audio", name: channelSetName(group), meta: channelSetMeta(files), path: null, channelFiles: files };
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

// sets are the ones mergeChannelSets just formed from groups
export function channelSetStatus(sets, groups) {
  const summary = `Channel set: ${sets.map((set) => `${set.name} (${set.meta})`).join(", ")}`;
  const setPaths = new Set(sets.flatMap((set) => set.channelFiles.map((file) => file.path)));
  const unnamed = groups.flatMap((group) => group.files).filter((file) => !file.laneInName && setPaths.has(file.path)).length;
  if (unnamed === 0) return summary;
  const files = unnamed === 1
    ? "1 file has no channel in its name and went on C. Set it in Channel mapping"
    : `${unnamed} files have no channel in their names and went on C. Set them in Channel mapping`;
  return `${summary}. ${files}`;
}
