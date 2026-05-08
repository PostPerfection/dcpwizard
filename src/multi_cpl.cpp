#include "dcpwizard/multi_cpl.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

std::vector<std::string> list_cpls(const std::filesystem::path& dcp_dir)
{
  spdlog::info("Listing CPLs in: {}", dcp_dir.string());
  // TODO: parse ASSETMAP and find CPL files
  return {};
}

std::vector<TimelineEntry> get_timeline(const std::filesystem::path& dcp_dir,
                                        const std::string& cpl_id)
{
  spdlog::info("Getting timeline for CPL: {}", cpl_id);
  // TODO: parse CPL XML for reel structure
  return {};
}

int create_multi_cpl(const MultiCPLConfig& config)
{
  spdlog::info("Creating multi-CPL DCP with {} compositions",
               config.cpl_ids.size());
  // TODO: implement multi-CPL DCP creation
  return 0;
}

} // namespace dcpwizard
