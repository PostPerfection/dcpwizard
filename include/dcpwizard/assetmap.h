#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct AssetMapEntry
{
  std::string id;
  std::filesystem::path path;
};

/// Generate an ASSETMAP/VOLINDEX for the DCP.
int generate_assetmap(const std::vector<AssetMapEntry>& entries,
                      const std::filesystem::path& output_dir);

} // namespace dcpwizard
