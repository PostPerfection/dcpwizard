#include "dcpwizard/verify.h"
#include "dcpwizard/hash.h"

#include <filesystem>
#include <fstream>
#include <spdlog/spdlog.h>
#include <string>

namespace fs = std::filesystem;

namespace dcpwizard
{

VerifyResult verify_dcp(const fs::path& dcp_dir)
{
  spdlog::info("Verifying DCP: {}", dcp_dir.string());
  VerifyResult result;

  if (!fs::exists(dcp_dir) || !fs::is_directory(dcp_dir))
  {
    result.errors.push_back("DCP directory does not exist: " + dcp_dir.string());
    return result;
  }

  // Check required files exist
  auto assetmap = dcp_dir / "ASSETMAP.xml";
  auto volindex = dcp_dir / "VOLINDEX.xml";

  if (!fs::exists(assetmap) && !fs::exists(dcp_dir / "ASSETMAP"))
  {
    result.errors.push_back("Missing ASSETMAP.xml");
  }
  if (!fs::exists(volindex) && !fs::exists(dcp_dir / "VOLINDEX"))
  {
    result.errors.push_back("Missing VOLINDEX.xml");
  }

  // Find PKL and CPL
  bool found_pkl = false;
  bool found_cpl = false;
  bool found_mxf = false;

  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    auto name = entry.path().filename().string();
    if (name.find("pkl") != std::string::npos || name.find("PKL") != std::string::npos)
      found_pkl = true;
    if (name.find("cpl") != std::string::npos || name.find("CPL") != std::string::npos)
      found_cpl = true;
    if (entry.path().extension() == ".mxf")
      found_mxf = true;
  }

  if (!found_pkl)
    result.errors.push_back("No PKL found");
  if (!found_cpl)
    result.errors.push_back("No CPL found");
  if (!found_mxf)
    result.warnings.push_back("No MXF track files found");

  // Verify PKL hashes
  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    if (entry.path().extension() == ".mxf")
    {
      auto hash = hash_file_base64(entry.path());
      if (hash.empty())
        result.warnings.push_back("Could not compute hash for: " + entry.path().filename().string());
    }
  }

  result.passed = result.errors.empty();

  if (result.passed)
    spdlog::info("DCP verification passed");
  else
    spdlog::error("DCP verification failed with {} errors", result.errors.size());

  return result;
}

} // namespace dcpwizard
