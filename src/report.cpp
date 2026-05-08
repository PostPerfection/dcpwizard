#include "dcpwizard/report.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int generate_report(const std::filesystem::path& dcp_dir,
                    const std::filesystem::path& output_html)
{
  spdlog::info("Generating QC report: {} → {}", dcp_dir.string(),
               output_html.string());
  // TODO: implement HTML report generation
  return 0;
}

} // namespace dcpwizard
