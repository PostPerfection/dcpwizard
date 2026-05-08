#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

/// Generate an HTML QC report for a DCP.
int generate_report(const std::filesystem::path& dcp_dir,
                    const std::filesystem::path& output_html);

} // namespace dcpwizard
