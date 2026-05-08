#include "dcpwizard/import.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int import_video(const ImportConfig& config)
{
  spdlog::info("Importing video: {} → {}", config.input_file.string(),
               config.output_dir.string());
  // TODO: implement via ffmpeg frame extraction
  return 0;
}

std::vector<std::string> supported_formats()
{
  return {"mov", "mp4", "mxf", "avi", "mkv", "mj2"};
}

} // namespace dcpwizard
