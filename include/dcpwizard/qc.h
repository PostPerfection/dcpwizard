#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class QCLevel
{
  Error,
  Warning,
  Info
};

struct QCResult
{
  QCLevel level;
  std::string code;
  std::string message;
  std::string location;  // e.g. reel/asset reference
};

struct QCReport
{
  bool passed = false;
  std::vector<QCResult> results;
  std::string standard;   // "SMPTE Bv2.1", "Interop", etc.
  uint64_t duration_ms = 0;
};

/// Run integrated quality control on a DCP (via dcpdoctor).
QCReport run_qc(const std::filesystem::path& dcp_dir);

} // namespace dcpwizard
