#include "dcpwizard/qc.h"
#include "dcpwizard/verify.h"

#include <chrono>
#include <filesystem>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

QCReport run_qc(const fs::path& dcp_dir)
{
  spdlog::info("Running QC on: {}", dcp_dir.string());
  auto start = std::chrono::steady_clock::now();
  QCReport report;
  report.standard = "SMPTE Bv2.1";

  if (!fs::exists(dcp_dir))
  {
    QCResult r;
    r.level = QCLevel::Error;
    r.code = "DIR_NOT_FOUND";
    r.message = "DCP directory does not exist";
    r.location = dcp_dir.string();
    report.results.push_back(r);
    report.passed = false;
    return report;
  }

  // Run structural verification
  auto verify = verify_dcp(dcp_dir);

  for (const auto& err : verify.errors)
  {
    QCResult r;
    r.level = QCLevel::Error;
    r.code = "STRUCTURE";
    r.message = err;
    report.results.push_back(r);
  }
  for (const auto& warn : verify.warnings)
  {
    QCResult r;
    r.level = QCLevel::Warning;
    r.code = "STRUCTURE";
    r.message = warn;
    report.results.push_back(r);
  }

  // Check MXF file sizes are reasonable
  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    if (entry.path().extension() == ".mxf")
    {
      auto size = fs::file_size(entry.path());
      if (size < 1024)
      {
        QCResult r;
        r.level = QCLevel::Warning;
        r.code = "MXF_SIZE";
        r.message = "MXF file suspiciously small (" + std::to_string(size) + " bytes)";
        r.location = entry.path().filename().string();
        report.results.push_back(r);
      }
    }
  }

  auto end = std::chrono::steady_clock::now();
  report.duration_ms = std::chrono::duration_cast<std::chrono::milliseconds>(end - start).count();
  report.passed = verify.passed;

  spdlog::info("QC complete: {} ({}ms, {} notes)",
               report.passed ? "PASS" : "FAIL",
               report.duration_ms, report.results.size());
  return report;
}

} // namespace dcpwizard
