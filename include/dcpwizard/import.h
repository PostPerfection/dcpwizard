#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct ImportConfig
{
  std::filesystem::path input_file;       // .mov, .mp4, .mxf, etc.
  std::filesystem::path output_dir;       // where to write extracted frames
  std::string pixel_format = "rgb48le";
  bool extract_audio = true;
  std::filesystem::path audio_output;     // WAV output path
};

/// Import a QuickTime/video container, extracting image sequence + audio.
int import_video(const ImportConfig& config);

/// Get list of supported container formats.
std::vector<std::string> supported_formats();

} // namespace dcpwizard
