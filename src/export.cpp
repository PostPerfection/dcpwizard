#include "dcpwizard/export.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int export_dcp(const ExportConfig& config)
{
  spdlog::info("Exporting DCP: {} → {}", config.dcp_dir.string(),
               config.output_file.string());
  // TODO: implement DCP export to ProRes/H.264/etc.
  return 0;
}

int extract_frame(const std::filesystem::path& dcp_dir,
                  uint64_t frame_number,
                  const std::filesystem::path& output_image)
{
  spdlog::info("Extracting frame {} from {}", frame_number,
               dcp_dir.string());
  // TODO: decode J2K frame and write as image
  return 0;
}

} // namespace dcpwizard
