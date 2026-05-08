#include "dcpwizard/vf.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int create_vf(const VFConfig& config)
{
  spdlog::info("Creating VF referencing OV: {}", config.original_dcp.string());
  // TODO: implement Version File creation
  return 0;
}

} // namespace dcpwizard
