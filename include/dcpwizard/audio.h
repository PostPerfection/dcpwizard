#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class ChannelLayout
{
  Mono,
  Stereo,
  FiveOne,
  SevenOne,
  Atmos
};

struct AudioConfig
{
  std::vector<std::filesystem::path> input_files;
  ChannelLayout layout = ChannelLayout::FiveOne;
  uint32_t sample_rate = 48000;
  uint32_t bit_depth = 24;
  std::string language; // RFC 5646
};

/// Wrap audio into MXF for DCP.
int wrap_audio(const AudioConfig& config,
               const std::filesystem::path& output_mxf);

} // namespace dcpwizard
