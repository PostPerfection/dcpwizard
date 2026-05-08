#include "dcpwizard/qc.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

QCReport run_qc(const std::filesystem::path& dcp_dir)
{
  spdlog::info("Running QC on: {}", dcp_dir.string());
  QCReport report;
  // TODO: integrate dcpdoctor for full QC
  report.passed = true;
  return report;
}

} // namespace dcpwizard
