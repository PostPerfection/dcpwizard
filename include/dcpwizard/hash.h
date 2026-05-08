#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

/// Compute SHA-1 hash of a file, returned as hex string.
std::string hash_file(const std::filesystem::path& file);

} // namespace dcpwizard
