#pragma once
#include <cstdint>

#include <filesystem>
#include <functional>
#include <string>

namespace dcpwizard
{

struct TranscodeConfig
{
  std::filesystem::path input_file;
  std::filesystem::path output_dir;
  std::string output_format = "tiff";
  std::string pixel_format = "rgb48le";
  uint16_t bit_depth = 16;
  uint32_t threads = 0;
  std::function<void(uint32_t current, uint32_t total)> on_progress;
};

struct TranscodeResult
{
  std::filesystem::path output_dir;
  std::filesystem::path audio_file; // Extracted audio (WAV), empty if none
  uint32_t frame_count = 0;
  uint32_t width = 0;
  uint32_t height = 0;
  double fps = 0.0;
  bool success = false;
  std::string error;
};

/// Check if ffmpeg is available in PATH.
bool ffmpeg_available();

/// Check if a file is a video container (by extension).
bool is_video_file(const std::filesystem::path& file);

/// Transcode video file to image sequence (TIFF) for J2K encoding.
/// Also extracts audio to WAV if present.
TranscodeResult transcode_to_sequence(const TranscodeConfig& config);

} // namespace dcpwizard
