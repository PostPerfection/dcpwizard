#pragma once
#include <cstdint>

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct PKLEntry
{
  std::string id;
  std::string type;
  std::filesystem::path file;
  std::string hash;
  uint64_t size = 0;
};

/// Generate a Packing List XML.
int generate_pkl(const std::vector<PKLEntry>& entries,
                 const std::filesystem::path& output_file);

} // namespace dcpwizard
