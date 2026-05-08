#pragma once
#include <cstdint>

#include <filesystem>
#include <string>

namespace dcpwizard
{

/// Wrap a DTS:X audio bitstream into MXF for DCP packaging.
int wrap_dtsx(const std::filesystem::path& input_file,
              const std::filesystem::path& output_mxf,
              uint32_t frame_rate_num = 24,
              uint32_t frame_rate_den = 1);

} // namespace dcpwizard
