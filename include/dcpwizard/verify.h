#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct VerifyResult
{
  bool passed = false;
  std::vector<std::string> errors;
  std::vector<std::string> warnings;
};

/// Verify a DCP using dcpdoctor.
VerifyResult verify_dcp(const std::filesystem::path& dcp_dir);

} // namespace dcpwizard
