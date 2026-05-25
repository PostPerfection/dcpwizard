#pragma once
#include <cstdint>

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct PKLEntry
{
  std::string id;       // UUID of the asset
  std::string type;     // MIME type
  std::string hash;     // Base64 SHA-1
  uint64_t size = 0;
  std::string original_filename;
};

struct PKLConfig
{
  std::string id;           // PKL UUID
  std::string annotation;
  std::string issuer = "dcpwizard";
  std::string creator = "DCP Wizard 0.1.0";
  std::vector<PKLEntry> entries;
};

/// Generate a Packing List XML.
int generate_pkl(const PKLConfig& config,
                 const std::filesystem::path& output_file);

} // namespace dcpwizard
