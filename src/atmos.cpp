#include "dcpwizard/atmos.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int wrap_atmos(const std::filesystem::path& input_iab,
               const std::filesystem::path& output_mxf,
               uint32_t frame_rate_num, uint32_t frame_rate_den)
{
  spdlog::info("Wrapping Atmos IAB: {} → {}", input_iab.string(),
               output_mxf.string());
  // TODO: implement IAB/Atmos MXF wrapping
  return 0;
}

} // namespace dcpwizard
