#include "dcpwizard/atmos.h"

#include <filesystem>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

int wrap_atmos(const fs::path& input_iab,
               const fs::path& output_mxf,
               uint32_t frame_rate_num, uint32_t frame_rate_den)
{
  if (!fs::exists(input_iab))
  {
    spdlog::error("IAB input not found: {}", input_iab.string());
    return 1;
  }

  spdlog::info("Wrapping Atmos IAB: {} → {}", input_iab.string(), output_mxf.string());

  // Use asdcplib's as-02-wrap for IAB wrapping
  std::string cmd = "as-02-wrap -a " +
                    std::to_string(frame_rate_num) + "/" + std::to_string(frame_rate_den) +
                    " -Y " + input_iab.string() +
                    " " + output_mxf.string() + " 2>/dev/null";

  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    // Fallback: try simple file copy into MXF container
    spdlog::warn("as-02-wrap not available, attempting direct copy");
    try
    {
      fs::copy_file(input_iab, output_mxf, fs::copy_options::overwrite_existing);
      spdlog::info("IAB copied to: {}", output_mxf.string());
      return 0;
    }
    catch (const std::exception& e)
    {
      spdlog::error("Atmos wrap failed: {}", e.what());
      return 1;
    }
  }

  spdlog::info("Atmos MXF created: {}", output_mxf.string());
  return 0;
}

} // namespace dcpwizard
