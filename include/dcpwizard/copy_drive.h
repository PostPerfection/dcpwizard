#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

/// Copy a DCP to a mounted drive (USB/CRU), verifying hashes after copy.
int copy_to_drive(const std::filesystem::path& dcp_dir,
                  const std::filesystem::path& destination);

} // namespace dcpwizard
