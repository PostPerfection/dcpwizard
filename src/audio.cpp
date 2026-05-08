#include "dcpwizard/audio.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int wrap_audio(const AudioConfig& config, const std::filesystem::path& output_mxf)
{
  spdlog::info("Wrapping {} audio file(s) → {}", config.input_files.size(),
               output_mxf.string());
  // TODO: implement audio MXF wrapping
  return 0;
}

} // namespace dcpwizard
