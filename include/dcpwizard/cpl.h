#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct ReelAsset
{
  std::string id;          // UUID for this asset reference
  std::string asset_id;    // UUID of the MXF file
  std::string hash;        // SHA-1 hash (base64)
  uint64_t size = 0;       // file size in bytes
  uint32_t duration = 0;   // in edit units (frames)
  uint32_t entry_point = 0;
  uint32_t frame_rate_num = 24;
  uint32_t frame_rate_den = 1;
};

struct CPLReel
{
  std::string id;             // UUID for this reel
  ReelAsset picture;
  ReelAsset sound;            // optional
};

struct CPLConfig
{
  std::string id;             // CPL UUID
  std::string title;
  std::string content_kind = "feature";
  std::string rating;
  std::string annotation;
  uint32_t frame_rate_num = 24;
  uint32_t frame_rate_den = 1;
  std::vector<CPLReel> reels;
};

/// Generate a Composition Playlist XML.
int generate_cpl(const CPLConfig& config,
                 const std::filesystem::path& output_file);

} // namespace dcpwizard
