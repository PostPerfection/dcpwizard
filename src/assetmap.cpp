#include "dcpwizard/assetmap.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int generate_assetmap(const std::vector<AssetMapEntry>& entries,
                      const std::filesystem::path& output_dir)
{
  spdlog::info("Generating ASSETMAP with {} entries", entries.size());
  // TODO: implement ASSETMAP/VOLINDEX generation
  return 0;
}

} // namespace dcpwizard
