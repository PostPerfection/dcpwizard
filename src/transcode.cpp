#include "dcpwizard/transcode.h"

#include <algorithm>
#include <array>
#include <cstdio>
#include <filesystem>
#include <spdlog/spdlog.h>
#include <sstream>
#include <string>

#ifdef _WIN32
#define popen _popen
#define pclose _pclose
#endif

namespace fs = std::filesystem;

namespace dcpwizard
{

static std::string exec_cmd(const std::string& cmd)
{
  std::array<char, 4096> buffer;
  std::string result;
  FILE* pipe = popen(cmd.c_str(), "r");
  if (!pipe)
    return {};
  while (fgets(buffer.data(), buffer.size(), pipe))
    result += buffer.data();
  pclose(pipe);
  while (!result.empty() && (result.back() == '\n' || result.back() == '\r'))
    result.pop_back();
  return result;
}

bool ffmpeg_available()
{
  return system("which ffmpeg >/dev/null 2>&1") == 0;
}

bool is_video_file(const fs::path& file)
{
  if (!fs::is_regular_file(file))
    return false;
  auto ext = file.extension().string();
  std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);
  return ext == ".mp4" || ext == ".mov" || ext == ".mkv" || ext == ".mxf" ||
         ext == ".avi" || ext == ".ts" || ext == ".m2ts";
}

TranscodeResult transcode_to_sequence(const TranscodeConfig& config)
{
  TranscodeResult result;

  if (!ffmpeg_available())
  {
    result.error = "ffmpeg not found in PATH";
    return result;
  }

  if (!fs::exists(config.input_file))
  {
    result.error = "Input file does not exist: " + config.input_file.string();
    return result;
  }

  fs::create_directories(config.output_dir);

  // Probe video info
  auto w_str = exec_cmd("ffprobe -v error -select_streams v:0 "
                        "-show_entries stream=width "
                        "-of default=noprint_wrappers=1:nokey=1 \"" +
                        config.input_file.string() + "\" 2>/dev/null");
  auto h_str = exec_cmd("ffprobe -v error -select_streams v:0 "
                        "-show_entries stream=height "
                        "-of default=noprint_wrappers=1:nokey=1 \"" +
                        config.input_file.string() + "\" 2>/dev/null");
  auto fps_str = exec_cmd("ffprobe -v error -select_streams v:0 "
                          "-show_entries stream=r_frame_rate "
                          "-of default=noprint_wrappers=1:nokey=1 \"" +
                          config.input_file.string() + "\" 2>/dev/null");

  try
  {
    result.width = static_cast<uint32_t>(std::stoul(w_str));
    result.height = static_cast<uint32_t>(std::stoul(h_str));
  }
  catch (...)
  {
  }

  auto slash = fps_str.find('/');
  if (slash != std::string::npos)
  {
    try
    {
      double num = std::stod(fps_str.substr(0, slash));
      double den = std::stod(fps_str.substr(slash + 1));
      if (den > 0)
        result.fps = num / den;
    }
    catch (...)
    {
    }
  }

  // Check if audio stream exists
  auto audio_codec =
      exec_cmd("ffprobe -v error -select_streams a:0 "
               "-show_entries stream=codec_name "
               "-of default=noprint_wrappers=1:nokey=1 \"" +
               config.input_file.string() + "\" 2>/dev/null");
  bool has_audio = !audio_codec.empty();

  // Determine pixel format
  std::string pix_fmt = config.pixel_format;
  if (pix_fmt.empty())
    pix_fmt = (config.bit_depth > 8) ? "rgb48le" : "rgb24";

  std::string ext = config.output_format.empty() ? "tiff" : config.output_format;

  // Get total frame count
  uint32_t total_frames = 0;
  auto frames_str =
      exec_cmd("ffprobe -v error -count_frames -select_streams v:0 "
               "-show_entries stream=nb_read_frames "
               "-of default=noprint_wrappers=1:nokey=1 \"" +
               config.input_file.string() + "\" 2>/dev/null");
  try
  {
    total_frames = static_cast<uint32_t>(std::stoul(frames_str));
  }
  catch (...)
  {
  }

  spdlog::info("Input: {}x{} @ {:.2f} fps, ~{} frames{}",
               result.width, result.height, result.fps, total_frames,
               has_audio ? " (with audio)" : "");

  // Extract video frames
  std::ostringstream cmd;
  cmd << "ffmpeg -y -i \"" << config.input_file.string() << "\"";
  cmd << " -pix_fmt " << pix_fmt;
  if (config.threads > 0)
    cmd << " -threads " << config.threads;
  auto frame_pattern = config.output_dir / ("frame_%06d." + ext);
  cmd << " \"" << frame_pattern.string() << "\" 2>/dev/null";

  spdlog::info("Extracting frames...");
  int ret = system(cmd.str().c_str());
  if (ret != 0)
  {
    result.error = "ffmpeg frame extraction failed with code " + std::to_string(ret);
    return result;
  }

  // Count output frames
  uint32_t count = 0;
  for (const auto& entry : fs::directory_iterator(config.output_dir))
  {
    if (entry.is_regular_file() && entry.path().extension() == ("." + ext))
      ++count;
  }
  result.frame_count = count;

  // Extract audio to WAV if present
  if (has_audio)
  {
    auto wav_path = config.output_dir / "audio.wav";
    std::ostringstream acmd;
    acmd << "ffmpeg -y -i \"" << config.input_file.string() << "\"";
    acmd << " -vn -acodec pcm_s24le -ar 48000";
    acmd << " \"" << wav_path.string() << "\" 2>/dev/null";

    spdlog::info("Extracting audio...");
    int arc = system(acmd.str().c_str());
    if (arc == 0 && fs::exists(wav_path))
    {
      result.audio_file = wav_path;
      spdlog::info("Audio extracted: {}", wav_path.string());
    }
    else
    {
      spdlog::warn("Audio extraction failed (non-fatal)");
    }
  }

  result.output_dir = config.output_dir;
  result.success = true;
  spdlog::info("Transcoded {} frames to {}", count, config.output_dir.string());
  return result;
}

} // namespace dcpwizard
