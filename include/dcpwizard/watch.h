#pragma once

#include <filesystem>
#include <functional>
#include <string>

namespace dcpwizard
{

using WatchCallback = std::function<void(const std::filesystem::path&)>;

/// Watch a directory for new files and trigger DCP creation.
int watch_directory(const std::filesystem::path& dir, WatchCallback callback);

} // namespace dcpwizard
