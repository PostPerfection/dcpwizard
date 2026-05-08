#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

/// Burn subtitles permanently into video frames.
int burnin(const std::filesystem::path& input_dir,
           const std::filesystem::path& subtitle_file,
           const std::filesystem::path& output_dir);

} // namespace dcpwizard
