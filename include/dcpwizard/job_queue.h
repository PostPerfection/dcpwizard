#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class JobType
{
  Transcode,
  Encode,
  Create,
  Validate,
  Loudness,
  QC,
  Copy,
  KDM
};

enum class JobState
{
  Queued,
  Running,
  Completed,
  Failed,
  Cancelled
};

struct Job
{
  uint64_t id = 0;
  JobType type = JobType::Transcode;
  JobState state = JobState::Queued;
  std::string description;
  std::vector<std::string> args;
  float progress = 0.0f;
  std::optional<std::chrono::steady_clock::time_point> started;
  std::optional<std::chrono::steady_clock::time_point> finished;
  std::string error_message;
};

using ProgressCallback = std::function<void(uint64_t job_id, float progress)>;

/// Start the job queue processing thread.
void start_job_queue(const std::filesystem::path& socket_path = "/tmp/dcpwizard.sock");

/// Stop the job queue.
void stop_job_queue();

/// Submit a job and return its ID.
uint64_t submit_job(const Job& job);

/// Cancel a queued or running job.
bool cancel_job(uint64_t job_id);

/// Query job status.
std::optional<Job> get_job(uint64_t job_id);

/// List all jobs.
std::vector<Job> list_jobs();

/// Convert JobType to string.
std::string job_type_to_string(JobType type);

/// Convert string to JobType.
JobType job_type_from_string(const std::string& s);

/// Convert JobState to string.
std::string job_state_to_string(JobState state);

/// Convert string to JobState.
JobState job_state_from_string(const std::string& s);

} // namespace dcpwizard
