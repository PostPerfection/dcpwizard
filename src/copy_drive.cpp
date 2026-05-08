#include "dcpwizard/copy_drive.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int copy_to_drive(const std::filesystem::path& dcp_dir,
                  const std::filesystem::path& destination)
{
  spdlog::info("Copying DCP to drive: {} → {}", dcp_dir.string(),
               destination.string());
  // TODO: implement copy with hash verification
  return 0;
}

} // namespace dcpwizard
