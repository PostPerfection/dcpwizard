#include "dcpwizard/geometry.h"

#include <filesystem>
#include <spdlog/spdlog.h>
#include <sstream>

namespace fs = std::filesystem;

namespace dcpwizard
{

int apply_geometry(const fs::path& input_dir,
                   const fs::path& output_dir,
                   const GeometryConfig& config)
{
  if (!fs::exists(input_dir))
  {
    spdlog::error("Input directory not found: {}", input_dir.string());
    return 1;
  }
  if (config.mode == ScaleMode::None)
  {
    spdlog::info("No geometry transform needed");
    return 0;
  }

  fs::create_directories(output_dir);

  // Build ffmpeg video filter
  std::string vf;
  switch (config.mode)
  {
    case ScaleMode::Scale:
      vf = "scale=" + std::to_string(config.target_width) + ":" +
           std::to_string(config.target_height);
      break;
    case ScaleMode::Crop:
    {
      std::ostringstream ss;
      ss << "crop=iw-" << (config.crop_left + config.crop_right)
         << ":ih-" << (config.crop_top + config.crop_bottom)
         << ":" << config.crop_left << ":" << config.crop_top;
      vf = ss.str();
      break;
    }
    case ScaleMode::Letterbox:
      vf = "scale=" + std::to_string(config.target_width) + ":" +
           std::to_string(config.target_height) +
           ":force_original_aspect_ratio=decrease,pad=" +
           std::to_string(config.target_width) + ":" +
           std::to_string(config.target_height) + ":(ow-iw)/2:(oh-ih)/2";
      break;
    case ScaleMode::PillarBox:
      vf = "scale=" + std::to_string(config.target_width) + ":" +
           std::to_string(config.target_height) +
           ":force_original_aspect_ratio=decrease,pad=" +
           std::to_string(config.target_width) + ":" +
           std::to_string(config.target_height) + ":(ow-iw)/2:(oh-ih)/2";
      break;
    default:
      return 0;
  }

  spdlog::info("Applying geometry: {} ({}x{})", vf, config.target_width, config.target_height);

  std::string cmd = "ffmpeg -y -framerate 24 -i \"" +
                    (input_dir / "frame_%06d.tiff").string() +
                    "\" -vf \"" + vf + "\" -pix_fmt rgb48le \"" +
                    (output_dir / "frame_%06d.tiff").string() +
                    "\" 2>/dev/null";

  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    spdlog::error("ffmpeg geometry transform failed");
    return 1;
  }

  uint32_t count = 0;
  for (const auto& entry : fs::directory_iterator(output_dir))
    if (entry.path().extension() == ".tiff")
      ++count;

  spdlog::info("Geometry applied: {} frames in {}", count, output_dir.string());
  return 0;
}

} // namespace dcpwizard
