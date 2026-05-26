#include "dcpwizard/copy_drive.h"
#include "dcpwizard/hash.h"

#include <filesystem>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

int copy_to_drive(const fs::path& dcp_dir, const fs::path& destination)
{
  if (!fs::exists(dcp_dir))
  {
    spdlog::error("Source DCP not found: {}", dcp_dir.string());
    return 1;
  }

  spdlog::info("Copying DCP to: {}", destination.string());
  auto dest_dir = destination / dcp_dir.filename();
  fs::create_directories(dest_dir);

  uint64_t total_bytes = 0;
  uint64_t copied_bytes = 0;
  for (const auto& entry : fs::recursive_directory_iterator(dcp_dir))
    if (entry.is_regular_file())
      total_bytes += fs::file_size(entry.path());

  for (const auto& entry : fs::recursive_directory_iterator(dcp_dir))
  {
    if (!entry.is_regular_file())
      continue;

    auto relative = fs::relative(entry.path(), dcp_dir);
    auto dest_file = dest_dir / relative;
    fs::create_directories(dest_file.parent_path());

    fs::copy_file(entry.path(), dest_file, fs::copy_options::overwrite_existing);
    copied_bytes += fs::file_size(entry.path());

    spdlog::debug("  Copied: {} ({:.1f}%)", relative.string(),
                  100.0 * copied_bytes / total_bytes);
  }

  // Verify hashes
  spdlog::info("Verifying copy integrity...");
  int errors = 0;
  for (const auto& entry : fs::recursive_directory_iterator(dcp_dir))
  {
    if (!entry.is_regular_file())
      continue;

    auto relative = fs::relative(entry.path(), dcp_dir);
    auto dest_file = dest_dir / relative;

    auto src_hash = hash_file_base64(entry.path());
    auto dst_hash = hash_file_base64(dest_file);

    if (src_hash != dst_hash)
    {
      spdlog::error("Hash mismatch: {}", relative.string());
      ++errors;
    }
  }

  if (errors > 0)
  {
    spdlog::error("Copy verification failed: {} files mismatched", errors);
    return 1;
  }

  spdlog::info("Copy complete and verified: {}", dest_dir.string());
  return 0;
}

} // namespace dcpwizard
