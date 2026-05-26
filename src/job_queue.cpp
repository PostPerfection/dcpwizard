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

  g_worker = std::thread([]{
    while (g_running)
    {
      Job* next = nullptr;
      {
        std::lock_guard lock(g_mutex);
        for (auto& j : g_jobs)
        {
          if (j.state == JobState::Queued)
          {
            j.state = JobState::Running;
            next = &j;
            break;
          }
        }
      }

      if (!next)
      {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        continue;
      }

      spdlog::info("Processing job {} ({})", next->id, job_type_to_string(next->type));

      // Execute job based on type (placeholder — real dispatch would call actual functions)
      next->progress = 100;
      next->state = JobState::Completed;
      spdlog::info("Job {} completed", next->id);
    }
  });
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

std::string job_type_to_string(JobType type)
{
  switch (type)
  {
    case JobType::Transcode: return "transcode";
    case JobType::Encode:    return "encode";
    case JobType::Create:    return "create";
    case JobType::Validate:  return "validate";
    case JobType::Loudness:  return "loudness";
    case JobType::QC:        return "qc";
    case JobType::Copy:      return "copy";
    case JobType::KDM:       return "kdm";
  }
  return "unknown";
}

JobType job_type_from_string(const std::string& s)
{
  if (s == "transcode") return JobType::Transcode;
  if (s == "encode")    return JobType::Encode;
  if (s == "create")    return JobType::Create;
  if (s == "validate")  return JobType::Validate;
  if (s == "loudness")  return JobType::Loudness;
  if (s == "qc")        return JobType::QC;
  if (s == "copy")      return JobType::Copy;
  if (s == "kdm")       return JobType::KDM;
  return JobType::Transcode;
}

std::string job_state_to_string(JobState state)
{
  switch (state)
  {
    case JobState::Queued:    return "queued";
    case JobState::Running:   return "running";
    case JobState::Completed: return "completed";
    case JobState::Failed:    return "failed";
    case JobState::Cancelled: return "cancelled";
  }
  return "unknown";
}

JobState job_state_from_string(const std::string& s)
{
  if (s == "queued")    return JobState::Queued;
  if (s == "running")   return JobState::Running;
  if (s == "completed") return JobState::Completed;
  if (s == "failed")    return JobState::Failed;
  if (s == "cancelled") return JobState::Cancelled;
  return JobState::Queued;
}

} // namespace dcpwizard
