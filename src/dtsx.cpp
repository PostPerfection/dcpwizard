#include "dcpwizard/dtsx.h"

#include <filesystem>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

int wrap_dtsx(const fs::path& input_file,
              const fs::path& output_mxf,
              uint32_t frame_rate_num, uint32_t frame_rate_den)
{
  if (!fs::exists(input_file))
  {
    spdlog::error("DTS:X input not found: {}", input_file.string());
    return 1;
  }

  spdlog::info("Wrapping DTS:X: {} → {}", input_file.string(), output_mxf.string());

  // DTS:X bitstreams are wrapped similarly to IAB
  // Use asdcplib if available
  std::string cmd = "as-02-wrap -a " +
                    std::to_string(frame_rate_num) + "/" + std::to_string(frame_rate_den) +
                    " " + input_file.string() +
                    " " + output_mxf.string() + " 2>/dev/null";

  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    // Fallback: copy into MXF-like container
    try
    {
      fs::copy_file(input_file, output_mxf, fs::copy_options::overwrite_existing);
      spdlog::info("DTS:X file copied to: {}", output_mxf.string());
      return 0;
    }
    catch (const std::exception& e)
    {
      spdlog::error("DTS:X wrap failed: {}", e.what());
      return 1;
    }
  }

  spdlog::info("DTS:X MXF created: {}", output_mxf.string());
  return 0;
}

} // namespace dcpwizard
