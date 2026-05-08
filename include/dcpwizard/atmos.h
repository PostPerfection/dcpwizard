#pragma once
#include <cstdint>

#include <filesystem>

namespace dcpwizard
{

/// Package an IAB (Immersive Audio Bitstream) Atmos track into MXF.
int wrap_atmos(const std::filesystem::path& input_iab,
               const std::filesystem::path& output_mxf,
               uint32_t frame_rate_num = 24,
               uint32_t frame_rate_den = 1);

} // namespace dcpwizard
