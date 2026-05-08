#pragma once

#include <string>

namespace dcpwizard
{

struct Profile
{
  std::string name;
  std::string standard;      // "SMPTE" or "Interop"
  std::string resolution;    // "2K" or "4K"
  uint32_t frame_rate = 24;
  uint32_t max_bitrate_mbps = 250;
  bool require_encryption = false;
};

/// Get a built-in delivery profile by name.
Profile get_profile(const std::string& name);

} // namespace dcpwizard
