#include "dcpwizard/verify.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

VerifyResult verify_dcp(const std::filesystem::path& dcp_dir)
{
  spdlog::info("Verifying DCP: {}", dcp_dir.string());
  VerifyResult result;
  // TODO: integrate dcpdoctor for verification
  result.passed = true;
  return result;
}

} // namespace dcpwizard
