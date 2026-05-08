#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

/// Supported HFR frame rates.
enum class FrameRate
{
  FPS_24 = 24,
  FPS_25 = 25,
  FPS_30 = 30,
  FPS_48 = 48,
  FPS_60 = 60,
  FPS_96 = 96,
  FPS_120 = 120
};

/// Check if a frame rate is valid for a given standard.
bool is_valid_frame_rate(FrameRate fps, bool smpte);

/// Get all supported frame rates for the given standard.
std::vector<FrameRate> supported_frame_rates(bool smpte);

} // namespace dcpwizard
