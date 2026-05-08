#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct CPLConfig
{
  std::string title;
  std::string content_kind;  // "feature", "trailer", "advertisement", etc.
  std::string rating;
  std::vector<std::string> reel_ids;
};

/// Generate a Composition Playlist XML.
int generate_cpl(const CPLConfig& config,
                 const std::filesystem::path& output_file);

} // namespace dcpwizard
