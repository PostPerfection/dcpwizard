#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class SubtitleFormat
{
  SMPTE_XML,
  Interop_XML,
  SRT
};

struct SubtitleConfig
{
  std::filesystem::path input_file;
  SubtitleFormat output_format = SubtitleFormat::SMPTE_XML;
  std::string language;           // RFC 5646
  float font_size = 42.0f;
  std::string font_family;
};

/// Import and convert subtitles for DCP packaging.
int import_subtitles(const SubtitleConfig& config);

/// Burn subtitles into video frames.
int burnin_subtitles(const std::filesystem::path& video_dir,
                     const std::filesystem::path& subtitle_file,
                     const std::filesystem::path& output_dir);

} // namespace dcpwizard
