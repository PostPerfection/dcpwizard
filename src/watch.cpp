#include "dcpwizard/watch.h"

#include <atomic>
#include <chrono>
#include <filesystem>
#include <set>
#include <spdlog/spdlog.h>
#include <thread>

#ifdef __linux__
#include <poll.h>
#include <sys/inotify.h>
#include <unistd.h>
#endif

namespace fs = std::filesystem;

namespace dcpwizard
{

int watch_directory(const fs::path& dir, WatchCallback callback)
{
  if (!fs::exists(dir))
  {
    spdlog::error("Watch directory does not exist: {}", dir.string());
    return 1;
  }

  spdlog::info("Watching directory: {}", dir.string());

#ifdef __linux__
  int fd = inotify_init1(IN_NONBLOCK);
  if (fd < 0)
  {
    spdlog::error("inotify_init failed");
    return 1;
  }

  int wd = inotify_add_watch(fd, dir.c_str(), IN_CREATE | IN_MOVED_TO | IN_CLOSE_WRITE);
  if (wd < 0)
  {
    spdlog::error("inotify_add_watch failed for: {}", dir.string());
    close(fd);
    return 1;
  }

  std::set<std::string> seen;
  char buf[4096];

  while (true)
  {
    struct pollfd pfd = {fd, POLLIN, 0};
    int ret = poll(&pfd, 1, 1000); // 1s timeout
    if (ret <= 0)
      continue;

    ssize_t len = read(fd, buf, sizeof(buf));
    if (len <= 0)
      continue;

    for (ssize_t i = 0; i < len;)
    {
      auto* event = reinterpret_cast<struct inotify_event*>(buf + i);
      if (event->len > 0)
      {
        std::string name(event->name);
        if (seen.find(name) == seen.end())
        {
          seen.insert(name);
          auto path = dir / name;
          spdlog::info("New file detected: {}", path.string());
          if (callback)
            callback(path);
        }
      }
      i += sizeof(struct inotify_event) + event->len;
    }
  }

  close(fd);
#else
  // Polling fallback for non-Linux
  std::set<std::string> known;
  for (const auto& entry : fs::directory_iterator(dir))
    known.insert(entry.path().string());

  while (true)
  {
    std::this_thread::sleep_for(std::chrono::seconds(2));
    for (const auto& entry : fs::directory_iterator(dir))
    {
      if (known.find(entry.path().string()) == known.end())
      {
        known.insert(entry.path().string());
        spdlog::info("New file detected: {}", entry.path().string());
        if (callback)
          callback(entry.path());
      }
    }
  }
#endif

  return 0;
}

} // namespace dcpwizard
