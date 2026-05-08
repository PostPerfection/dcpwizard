#include "dcpwizard/watch.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int watch_directory(const std::filesystem::path& dir, WatchCallback callback)
{
  spdlog::info("Watching directory: {}", dir.string());
  // TODO: implement inotify/FSEvents directory watching
  return 0;
}

} // namespace dcpwizard
