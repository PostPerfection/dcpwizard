#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct VFConfig
{
  std::filesystem::path original_dcp;  // OV (Original Version)
  std::filesystem::path output_dir;
  std::vector<std::string> replaced_reels; // reel IDs to replace
};

/// Create a VF (Version File) DCP referencing an OV.
int create_vf(const VFConfig& config);

} // namespace dcpwizard
