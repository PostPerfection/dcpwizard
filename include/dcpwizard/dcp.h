#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class Standard
{
  SMPTE,
  Interop
};

enum class Resolution
{
  TwoK,   // 2048x1080
  FourK   // 4096x2160
};

struct DCPConfig
{
  std::string title;
  Standard standard = Standard::SMPTE;
  Resolution resolution = Resolution::TwoK;
  uint32_t frame_rate_num = 24;
  uint32_t frame_rate_den = 1;
  uint32_t max_bitrate_mbps = 250;  // up to 500 for HBR
  bool encrypt = false;
  bool stereo_3d = false;
  std::filesystem::path video_dir;   // input image sequence directory
  std::filesystem::path audio_file;  // optional WAV audio input
  std::filesystem::path output_dir;
};

/// Create a complete DCP from the given configuration.
int create_dcp(const DCPConfig& config);

} // namespace dcpwizard
