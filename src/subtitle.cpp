#include "dcpwizard/subtitle.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int import_subtitles(const SubtitleConfig& config)
{
  spdlog::info("Importing subtitles: {}", config.input_file.string());
  // TODO: implement subtitle import/conversion
  return 0;
}

int burnin_subtitles(const std::filesystem::path& video_dir,
                     const std::filesystem::path& subtitle_file,
                     const std::filesystem::path& output_dir)
{
  spdlog::info("Burning subtitles into frames");
  // TODO: implement subtitle burn-in
  return 0;
}

} // namespace dcpwizard
