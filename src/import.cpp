#include "dcpwizard/import.h"
#include "dcpwizard/transcode.h"

#include <filesystem>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

int import_video(const ImportConfig& config)
{
  if (!fs::exists(config.input_file))
  {
    spdlog::error("Input file not found: {}", config.input_file.string());
    return 1;
  }

  spdlog::info("Importing video: {}", config.input_file.string());

  TranscodeConfig tc;
  tc.input_file = config.input_file;
  tc.output_dir = config.output_dir;
  tc.pixel_format = config.pixel_format;

  auto result = transcode_to_sequence(tc);
  if (!result.success)
  {
    spdlog::error("Import failed: {}", result.error);
    return 1;
  }

  // Copy extracted audio if requested
  if (config.extract_audio && !result.audio_file.empty())
  {
    auto audio_dest = config.audio_output;
    if (audio_dest.empty())
      audio_dest = config.output_dir / "audio.wav";
    if (result.audio_file != audio_dest)
      fs::copy_file(result.audio_file, audio_dest, fs::copy_options::overwrite_existing);
    spdlog::info("Audio extracted: {}", audio_dest.string());
  }

  spdlog::info("Import complete: {} frames in {}", result.frame_count, config.output_dir.string());
  return 0;
}

std::vector<std::string> supported_formats()
{
  return {"mov", "mp4", "mxf", "avi", "mkv", "mj2", "ts", "m2ts"};
}

} // namespace dcpwizard
