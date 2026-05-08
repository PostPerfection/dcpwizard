#pragma once

#include <cstdint>
#include <filesystem>
#include <string>

namespace dcpwizard
{

enum class ScaleMode
{
  None,
  Scale,
  Crop,
  Letterbox,
  PillarBox
};

struct GeometryConfig
{
  ScaleMode mode = ScaleMode::None;
  uint32_t target_width = 2048;
  uint32_t target_height = 1080;
  uint32_t crop_left = 0;
  uint32_t crop_right = 0;
  uint32_t crop_top = 0;
  uint32_t crop_bottom = 0;
};

/// Apply scale/crop/letterbox to an image sequence.
int apply_geometry(const std::filesystem::path& input_dir,
                   const std::filesystem::path& output_dir,
                   const GeometryConfig& config);

} // namespace dcpwizard
