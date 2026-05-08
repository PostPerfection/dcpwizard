#pragma once

#include <chrono>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct KDMConfig
{
  std::filesystem::path dcp_dir;
  std::filesystem::path certificate;    // recipient certificate PEM
  std::string content_title;
  std::chrono::system_clock::time_point not_valid_before;
  std::chrono::system_clock::time_point not_valid_after;
  std::filesystem::path output_file;
};

/// Generate a KDM for an encrypted DCP.
int generate_kdm(const KDMConfig& config);

/// Batch-generate KDMs for multiple recipients.
int generate_kdm_batch(const std::filesystem::path& dcp_dir,
                       const std::vector<std::filesystem::path>& certificates,
                       const std::filesystem::path& output_dir);

} // namespace dcpwizard
