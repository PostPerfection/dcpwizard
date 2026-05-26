#include "dcpwizard/burnin.h"

#include <filesystem>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

int burnin(const fs::path& input_dir,
           const fs::path& subtitle_file,
           const fs::path& output_dir)
{
  if (!fs::exists(input_dir))
  {
    spdlog::error("Input directory not found: {}", input_dir.string());
    return 1;
  }
  if (!fs::exists(subtitle_file))
  {
    spdlog::error("Subtitle file not found: {}", subtitle_file.string());
    return 1;
  }

  fs::create_directories(output_dir);
  spdlog::info("Burning subtitles: {} + {} → {}",
               input_dir.string(), subtitle_file.string(), output_dir.string());

  // Use ffmpeg with image sequence input and subtitle overlay
  std::string cmd = "ffmpeg -y -framerate 24 -i \"" +
                    (input_dir / "frame_%06d.tiff").string() +
                    "\" -vf \"subtitles='" + subtitle_file.string() +
                    "'\" -pix_fmt rgb48le \"" +
                    (output_dir / "frame_%06d.tiff").string() +
                    "\" 2>/dev/null";

  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    spdlog::error("ffmpeg burn-in failed with exit code {}", rc);
    return 1;
  }

  uint32_t count = 0;
  for (const auto& entry : fs::directory_iterator(output_dir))
    if (entry.path().extension() == ".tiff")
      ++count;

  spdlog::info("Burn-in complete: {} frames in {}", count, output_dir.string());
  return 0;
}

} // namespace dcpwizard
