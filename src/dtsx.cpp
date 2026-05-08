#include "dcpwizard/dtsx.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int wrap_dtsx(const std::filesystem::path& input_file,
              const std::filesystem::path& output_mxf,
              uint32_t frame_rate_num, uint32_t frame_rate_den)
{
  spdlog::info("Wrapping DTS:X: {} → {}", input_file.string(),
               output_mxf.string());
  // TODO: implement DTS:X MXF wrapping
  return 0;
}

} // namespace dcpwizard
