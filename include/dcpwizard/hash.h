#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

/// Compute SHA-1 hash of a file, returned as base64 string (DCP standard).
std::string hash_file_base64(const std::filesystem::path& file);

/// Compute SHA-1 hash of a file, returned as hex string.
std::string hash_file(const std::filesystem::path& file);

} // namespace dcpwizard
