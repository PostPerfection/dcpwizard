#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

struct LoudnessResult
{
  float integrated_lufs = 0.0f;
  float true_peak_dbtp = 0.0f;
  float lra_lu = 0.0f;
  bool passed = false;
};

/// Measure loudness of audio file per EBU R128 / ATSC A/85.
LoudnessResult measure_loudness(const std::filesystem::path& audio_file);

} // namespace dcpwizard
