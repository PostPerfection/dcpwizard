#include "dcpwizard/mxf_wrap.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int wrap_mxf(const MXFWrapConfig& config)
{
  spdlog::info("Wrapping MXF: {} → {}", config.input.string(),
               config.output.string());
  // TODO: implement MXF wrapping via asdcplib
  return 0;
}

} // namespace dcpwizard
