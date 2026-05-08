#pragma once

#include <cstdint>
#include <filesystem>
#include <string>

namespace dcpwizard
{

enum class Encoder
{
  Grok,
  OpenJPEG
};

struct EncodeConfig
{
  Encoder encoder = Encoder::OpenJPEG;
  uint32_t bandwidth_mbps = 250;
  uint32_t threads = 0; // 0 = auto-detect
  std::filesystem::path input_dir;
  std::filesystem::path output_dir;
};

/// Encode image sequence to JPEG 2000 codestream.
int encode_j2k(const EncodeConfig& config);

} // namespace dcpwizard
