#include "dcpwizard/j2k_transcode.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int transcode_j2k(const J2KTranscodeConfig& config)
{
  spdlog::info("Transcoding J2K: {} → {} @ {} Mbps",
               config.input_dir.string(), config.output_dir.string(),
               config.target_bandwidth_mbps);
  // TODO: implement J2K re-encoding
  return 0;
}

} // namespace dcpwizard
