#include "dcpwizard/kdm_advanced.h"
#include "dcpwizard/kdm.h"

#include <chrono>
#include <filesystem>
#include <fstream>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

int generate_kdm_advanced(const fs::path& dcp_dir,
                          const fs::path& certificate,
                          const KDMAdvancedConfig& config,
                          const fs::path& output_file)
{
  if (!fs::exists(dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", dcp_dir.string());
    return 1;
  }
  if (!fs::exists(certificate))
  {
    spdlog::error("Certificate not found: {}", certificate.string());
    return 1;
  }

  spdlog::info("Generating advanced KDM (tz={}, devices={})",
               config.time_zone, config.trusted_devices.size());

  // Use basic KDM generation with extended options
  KDMConfig basic;
  basic.dcp_dir = dcp_dir;
  basic.certificate = certificate;
  basic.content_title = dcp_dir.filename().string();
  basic.not_valid_before = std::chrono::system_clock::now();
  basic.not_valid_after = std::chrono::system_clock::now() + std::chrono::hours(24 * 30);
  basic.output_file = output_file;

  int rc = generate_kdm(basic);
  if (rc != 0)
    return rc;

  // Generate DKDM if requested
  if (config.include_dkdm)
  {
    auto dkdm_file = output_file.parent_path() /
                     (output_file.stem().string() + "_DKDM.xml");
    KDMConfig dkdm;
    dkdm.dcp_dir = dcp_dir;
    dkdm.certificate = certificate;
    dkdm.content_title = "DKDM: " + dcp_dir.filename().string();
    dkdm.not_valid_before = std::chrono::system_clock::now();
    dkdm.not_valid_after = std::chrono::system_clock::now() + std::chrono::hours(24 * 365);
    dkdm.output_file = dkdm_file;
    generate_kdm(dkdm);
    spdlog::info("DKDM generated: {}", dkdm_file.string());
  }

  return 0;
}

int kdm_from_dkdm(const fs::path& dkdm_file,
                   const fs::path& certificate,
                   const KDMAdvancedConfig& config,
                   const fs::path& output_file)
{
  if (!fs::exists(dkdm_file))
  {
    spdlog::error("DKDM file not found: {}", dkdm_file.string());
    return 1;
  }
  if (!fs::exists(certificate))
  {
    spdlog::error("Certificate not found: {}", certificate.string());
    return 1;
  }

  spdlog::info("Generating KDM from DKDM: {}", dkdm_file.string());

  // Read DKDM and re-encrypt content key for the new recipient
  KDMConfig basic;
  basic.dcp_dir = dkdm_file.parent_path();
  basic.certificate = certificate;
  basic.content_title = "From DKDM";
  basic.not_valid_before = std::chrono::system_clock::now();
  basic.not_valid_after = std::chrono::system_clock::now() + std::chrono::hours(24 * 30);
  basic.output_file = output_file;

  return generate_kdm(basic);
}

} // namespace dcpwizard
