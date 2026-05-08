#include "dcpwizard/job_queue.h"
#include <spdlog/spdlog.h>

#include <mutex>
#include <queue>
#include <thread>
#include <atomic>

namespace dcpwizard
{

static std::mutex g_mutex;
static std::vector<Job> g_jobs;
static std::atomic<uint64_t> g_next_id{1};
static std::atomic<bool> g_running{false};
static std::thread g_worker;

void start_job_queue(const std::filesystem::path& socket_path)
{
  spdlog::info("Starting job queue (socket: {})", socket_path.string());
  g_running = true;
  // TODO: implement Unix domain socket listener + worker thread
}

void stop_job_queue()
{
  g_running = false;
  if (g_worker.joinable())
    g_worker.join();
}

uint64_t submit_job(const Job& job)
{
  std::lock_guard lock(g_mutex);
  Job j = job;
  j.id = g_next_id++;
  j.state = JobState::Queued;
  g_jobs.push_back(j);
  return j.id;
}

bool cancel_job(uint64_t job_id)
{
  std::lock_guard lock(g_mutex);
  for (auto& j : g_jobs)
  {
    if (j.id == job_id && j.state == JobState::Queued)
    {
      j.state = JobState::Cancelled;
      return true;
    }
  }
  return false;
}

std::optional<Job> get_job(uint64_t job_id)
{
  std::lock_guard lock(g_mutex);
  for (const auto& j : g_jobs)
    if (j.id == job_id)
      return j;
  return std::nullopt;
}

std::vector<Job> list_jobs()
{
  std::lock_guard lock(g_mutex);
  return g_jobs;
}

} // namespace dcpwizard
