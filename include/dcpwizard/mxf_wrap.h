#pragma once

#include <cstdint>
#include <filesystem>
#include <string>

namespace dcpwizard
{

enum class MXFType
{
  Picture,
  Sound,
  Subtitle,
  Atmos
};

struct MXFWrapConfig
{
  MXFType type = MXFType::Picture;
  std::filesystem::path input;
  std::filesystem::path output;
  uint32_t frame_rate_num = 24;
  uint32_t frame_rate_den = 1;
  bool encrypt = false;
  std::string key_id;
  std::string key;
};

/// Wrap essence into an MXF container.
int wrap_mxf(const MXFWrapConfig& config);

} // namespace dcpwizard
