#pragma once

#include <chrono>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct TrustedDevice
{
  std::string thumbprint;  // certificate thumbprint
  std::string description;
};

struct KDMAdvancedConfig
{
  std::string time_zone;              // e.g. "America/Los_Angeles"
  std::string annotation_scheme;      // custom annotation text pattern
  std::vector<TrustedDevice> trusted_devices;
  bool include_dkdm = false;         // generate DKDM alongside KDM
};

/// Generate KDM with advanced options (timezone, annotations, trusted devices).
int generate_kdm_advanced(const std::filesystem::path& dcp_dir,
                          const std::filesystem::path& certificate,
                          const KDMAdvancedConfig& config,
                          const std::filesystem::path& output_file);

/// Read DKDM and generate KDM from it (without needing original DCP).
int kdm_from_dkdm(const std::filesystem::path& dkdm_file,
                   const std::filesystem::path& certificate,
                   const KDMAdvancedConfig& config,
                   const std::filesystem::path& output_file);

} // namespace dcpwizard
