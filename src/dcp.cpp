#include "dcpwizard/dcp.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int create_dcp(const DCPConfig& config)
{
  spdlog::info("Creating DCP: {} ({})", config.title,
               config.standard == Standard::SMPTE ? "SMPTE" : "Interop");
  // TODO: implement full DCP creation pipeline
  return 0;
}

} // namespace dcpwizard
