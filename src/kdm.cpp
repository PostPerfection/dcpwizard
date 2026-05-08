#include "dcpwizard/kdm.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int generate_kdm(const KDMConfig& config)
{
  spdlog::info("Generating KDM for: {}", config.content_title);
  // TODO: implement KDM generation
  return 0;
}

int generate_kdm_batch(const std::filesystem::path& dcp_dir,
                       const std::vector<std::filesystem::path>& certificates,
                       const std::filesystem::path& output_dir)
{
  spdlog::info("Batch KDM generation: {} recipients", certificates.size());
  // TODO: implement batch KDM generation
  return 0;
}

} // namespace dcpwizard
