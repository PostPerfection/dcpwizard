#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct MultiCPLConfig
{
  std::filesystem::path dcp_dir;
  std::vector<std::string> cpl_ids;   // CPL UUIDs in this DCP
  std::string active_cpl;             // currently active for editing
};

struct TimelineEntry
{
  std::string reel_id;
  uint64_t start_frame = 0;
  uint64_t end_frame = 0;
  std::string asset_type;  // "picture", "sound", "subtitle"
};

/// List CPLs in a DCP.
std::vector<std::string> list_cpls(const std::filesystem::path& dcp_dir);

/// Get timeline entries for a CPL.
std::vector<TimelineEntry> get_timeline(const std::filesystem::path& dcp_dir,
                                        const std::string& cpl_id);

/// Create a DCP with multiple CPLs sharing reel assets.
int create_multi_cpl(const MultiCPLConfig& config);

} // namespace dcpwizard
