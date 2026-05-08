#include "dcpwizard/burnin.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int burnin(const std::filesystem::path& input_dir,
           const std::filesystem::path& subtitle_file,
           const std::filesystem::path& output_dir)
{
  spdlog::info("Burning subtitles into frames: {}", subtitle_file.string());
  // TODO: implement subtitle burn-in via ffmpeg
  return 0;
}

} // namespace dcpwizard
