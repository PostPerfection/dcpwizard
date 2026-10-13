import { filledRatings } from "./rating-rows.js";

export const KEEP_CONTAINER = "keep";
export const CUSTOM_CONTAINER = "custom";

const CONTAINER_SIZE = /^\d+[xX]\d+$/;
const STANDARD_NAMES = { Smpte: "SMPTE", Interop: "Interop" };
// what the CPL writer declares when the source CPL names no kind
const UNDECLARED_CONTENT_KIND = "feature";
const FRACTIONAL_FRAME_RATE_DECIMALS = 3;

export function rebuildRequestFrom(fields, joinedItems, ratingRows) {
  const refusals = [];
  if (!fields.source) refusals.push("Choose the DCP to rebuild");
  if (!fields.outputParent) refusals.push("Choose the folder the new DCP is written into");
  const title = fields.title.trim();
  if (!title) refusals.push("Give the new DCP a title");
  const custom = fields.container === CUSTOM_CONTAINER;
  const containerDims = fields.containerDims.trim();
  if (custom && !CONTAINER_SIZE.test(containerDims)) refusals.push("Give the custom container as WxH, such as 1920x1080");
  const bitRateText = fields.bitRate.trim();
  const bitRate = bitRateText === "" ? null : Number(bitRateText);
  if (bitRate !== null && !(Number.isInteger(bitRate) && bitRate > 0)) refusals.push("The bit rate has to be a whole number of Mbit/s above 0");

  if (refusals.length > 0) return { refusals };
  return {
    request: {
      source: fields.source,
      output_parent: fields.outputParent,
      title,
      head_items: [...joinedItems.head],
      tail_items: [...joinedItems.tail],
      ratings: filledRatings(ratingRows),
      container: fields.container,
      container_dims: custom ? containerDims : null,
      video_bit_rate_mbps: bitRate,
    },
  };
}

export function rebuildReady(fields, sourceReadable) {
  return Boolean(sourceReadable && fields.outputParent && fields.title.trim());
}

export function frameRateText(numerator, denominator) {
  if (denominator === 1) return `${numerator} fps`;
  return `${(numerator / denominator).toFixed(FRACTIONAL_FRAME_RATE_DECIMALS)} fps`;
}

export function sourceSummary(source) {
  return [
    `Title: ${source.title}`,
    `Standard: ${STANDARD_NAMES[source.standard] || source.standard}`,
    `Picture: ${source.stored_width}×${source.stored_height}, active area ${source.active_width}×${source.active_height}`,
    `Frame rate: ${frameRateText(source.edit_rate_numerator, source.edit_rate_denominator)}`,
    `Reels: ${source.reel_count}, ${source.total_frames} frames`,
    `Kind: ${source.content_kind || UNDECLARED_CONTENT_KIND}`,
  ].join("\n");
}

export function rebuildProgressText(progress) {
  if (progress.total_frames > 0) return `${progress.message}, frame ${progress.frame} / ${progress.total_frames}`;
  return progress.message;
}
