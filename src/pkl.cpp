#include "dcpwizard/pkl.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int generate_pkl(const std::vector<PKLEntry>& entries,
                 const std::filesystem::path& output_file)
{
  spdlog::info("Generating PKL with {} entries", entries.size());
  // TODO: implement PKL XML generation
  return 0;
}

} // namespace dcpwizard
