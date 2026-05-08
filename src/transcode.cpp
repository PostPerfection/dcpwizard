#include "dcpwizard/transcode.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int transcode_to_sequence(const TranscodeConfig& config)
{
  spdlog::info("Transcoding {} → {}", config.input_file.string(),
               config.output_dir.string());
  // TODO: implement ffmpeg-based transcoding
  return 0;
}

} // namespace dcpwizard
