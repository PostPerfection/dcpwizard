#pragma once
#include <cstdint>

#include <filesystem>
#include <string>

namespace dcpwizard
{

struct DCPInfo
{
  std::string title;
  std::string standard;
  std::string resolution;
  std::string frame_rate;
  uint64_t duration_frames = 0;
  uint64_t total_size_bytes = 0;
  uint32_t reel_count = 0;
  bool encrypted = false;
  bool stereo_3d = false;
};

/// Read metadata from an existing DCP.
DCPInfo inspect_dcp(const std::filesystem::path& dcp_dir);

} // namespace dcpwizard
