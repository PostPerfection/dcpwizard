#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct AssetMapEntry
{
  std::string id;         // UUID of the asset
  std::string path;       // relative file path within DCP directory
  uint64_t size = 0;
};

struct AssetMapConfig
{
  std::string id;         // ASSETMAP UUID
  std::string annotation;
  std::string issuer = "dcpwizard";
  std::string creator = "DCP Wizard 0.1.0";
  std::vector<AssetMapEntry> entries;
};

/// Generate ASSETMAP and VOLINDEX XML files.
int generate_assetmap(const AssetMapConfig& config,
                      const std::filesystem::path& output_dir);

} // namespace dcpwizard
