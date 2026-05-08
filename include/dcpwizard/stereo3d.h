#pragma once
#include <cstdint>

#include <filesystem>
#include <string>

namespace dcpwizard
{

enum class Eye
{
  Left,
  Right
};

struct Stereo3DConfig
{
  std::filesystem::path left_dir;
  std::filesystem::path right_dir;
  std::filesystem::path output_mxf;
  uint32_t frame_rate_num = 24;
  uint32_t frame_rate_den = 1;
};

/// Create a stereoscopic 3D picture MXF with interleaved frames.
int create_stereo3d(const Stereo3DConfig& config);

} // namespace dcpwizard
