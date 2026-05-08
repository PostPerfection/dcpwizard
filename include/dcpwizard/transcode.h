#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

struct TranscodeConfig
{
  std::filesystem::path input_file;
  std::filesystem::path output_dir;
  std::string pixel_format = "rgb48le";
  uint32_t threads = 0;
};

/// Transcode video file to image sequence (DPX/TIFF) for J2K encoding.
int transcode_to_sequence(const TranscodeConfig& config);

} // namespace dcpwizard
