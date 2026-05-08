#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class ReelSplitMode
{
  None,
  ByDuration,  // split every N minutes
  BySize       // split every N GB
};

struct ReelConfig
{
  ReelSplitMode split_mode = ReelSplitMode::None;
  uint32_t split_duration_minutes = 20;
  uint64_t split_size_bytes = 0;
};

struct ReelInfo
{
  std::string id;
  uint64_t frame_start = 0;
  uint64_t frame_end = 0;
  std::filesystem::path picture_mxf;
  std::filesystem::path sound_mxf;
  std::filesystem::path subtitle_mxf;
};

/// Plan reel splits for the given duration/size.
std::vector<ReelInfo> plan_reels(uint64_t total_frames,
                                 uint32_t frame_rate,
                                 const ReelConfig& config);

} // namespace dcpwizard
