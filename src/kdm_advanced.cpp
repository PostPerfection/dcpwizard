#include "dcpwizard/kdm_advanced.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int generate_kdm_advanced(const std::filesystem::path& dcp_dir,
                          const std::filesystem::path& certificate,
                          const KDMAdvancedConfig& config,
                          const std::filesystem::path& output_file)
{
  spdlog::info("Generating advanced KDM (tz={}, devices={})",
               config.time_zone, config.trusted_devices.size());
  // TODO: implement advanced KDM generation
  return 0;
}

int kdm_from_dkdm(const std::filesystem::path& dkdm_file,
                   const std::filesystem::path& certificate,
                   const KDMAdvancedConfig& config,
                   const std::filesystem::path& output_file)
{
  spdlog::info("Generating KDM from DKDM: {}", dkdm_file.string());
  // TODO: implement DKDM-based KDM generation
  return 0;
}

} // namespace dcpwizard
