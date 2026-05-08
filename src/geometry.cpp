#include "dcpwizard/geometry.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int apply_geometry(const std::filesystem::path& input_dir,
                   const std::filesystem::path& output_dir,
                   const GeometryConfig& config)
{
  spdlog::info("Applying geometry: {}x{} (mode={})", config.target_width,
               config.target_height, static_cast<int>(config.mode));
  // TODO: implement scale/crop/letterbox via ffmpeg or custom
  return 0;
}

} // namespace dcpwizard
