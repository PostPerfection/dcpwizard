#pragma once

#include <cstdint>
#include <filesystem>
#include <string>

namespace dcpwizard
{

struct J2KTranscodeConfig
{
  std::filesystem::path input_dir;   // existing J2K codestream files
  std::filesystem::path output_dir;
  uint32_t target_bandwidth_mbps = 250;
  bool decode_first = true;          // decode → re-encode (lossier but size control)
};

/// Re-encode/transcode existing JPEG 2000 at a different bitrate.
int transcode_j2k(const J2KTranscodeConfig& config);

} // namespace dcpwizard
