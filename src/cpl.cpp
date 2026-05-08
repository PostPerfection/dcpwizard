#include "dcpwizard/cpl.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int generate_cpl(const CPLConfig& config, const std::filesystem::path& output_file)
{
  spdlog::info("Generating CPL: {}", config.title);
  // TODO: implement CPL XML generation
  return 0;
}

} // namespace dcpwizard
