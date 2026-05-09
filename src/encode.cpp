#include "dcpwizard/encode.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int encode_j2k(const EncodeConfig& config)
{
  spdlog::info("Encoding J2K: {} → {} @ {} Mbps",
               config.input_dir.string(), config.output_dir.string(),
               config.bandwidth_mbps);
  // TODO: implement J2K encoding via grok
  return 0;
}

} // namespace dcpwizard
