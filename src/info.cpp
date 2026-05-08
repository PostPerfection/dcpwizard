#include "dcpwizard/info.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

DCPInfo inspect_dcp(const std::filesystem::path& dcp_dir)
{
  spdlog::info("Inspecting DCP: {}", dcp_dir.string());
  DCPInfo info;
  // TODO: implement DCP metadata reading
  return info;
}

} // namespace dcpwizard
