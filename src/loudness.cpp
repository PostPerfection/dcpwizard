#include "dcpwizard/loudness.h"

#include <array>
#include <cstdio>
#include <filesystem>
#include <regex>
#include <spdlog/spdlog.h>
#include <string>

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
  return result;
}

LoudnessResult measure_loudness(const fs::path& audio_file)
{
  LoudnessResult result;

  if (!fs::exists(audio_file))
  {
    spdlog::error("Audio file not found: {}", audio_file.string());
    return result;
  }

  spdlog::info("Measuring loudness: {}", audio_file.string());

  // Use ffmpeg's ebur128 filter
  std::string cmd = "ffmpeg -i \"" + audio_file.string() +
                    "\" -af ebur128=peak=true -f null - 2>&1";
  auto output = exec_cmd(cmd);

  if (output.empty())
  {
    spdlog::error("ffmpeg loudness measurement failed");
    return result;
  }

  // Parse integrated loudness
  std::regex integrated_re("I:\\s+(-?[\\d.]+)\\s+LUFS");
  std::regex peak_re("Peak:\\s+(-?[\\d.]+)\\s+dBFS");
  std::regex lra_re("LRA:\\s+(-?[\\d.]+)\\s+LU");

  std::smatch match;
  if (std::regex_search(output, match, integrated_re))
    result.integrated_lufs = std::stof(match[1].str());
  if (std::regex_search(output, match, peak_re))
    result.true_peak_dbtp = std::stof(match[1].str());
  if (std::regex_search(output, match, lra_re))
    result.lra_lu = std::stof(match[1].str());

  // SMPTE ST 2098-2 / EBU R128: cinema target is -24 LUFS (±2 LU)
  result.passed = (result.integrated_lufs >= -26.0f && result.integrated_lufs <= -22.0f);

  spdlog::info("  Integrated: {:.1f} LUFS", result.integrated_lufs);
  spdlog::info("  True Peak: {:.1f} dBTP", result.true_peak_dbtp);
  spdlog::info("  LRA: {:.1f} LU", result.lra_lu);
  spdlog::info("  Compliance: {}", result.passed ? "PASS" : "FAIL");

  return result;
}

} // namespace dcpwizard
