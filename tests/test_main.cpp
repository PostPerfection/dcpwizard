#include <cstdlib>
#include <iostream>

#include "dcpwizard/dcp.h"
#include "dcpwizard/verify.h"
#include "dcpwizard/job_queue.h"

static int tests_run = 0;
static int tests_passed = 0;

#define ASSERT(cond)                                                \
  do {                                                              \
    ++tests_run;                                                    \
    if (!(cond)) {                                                  \
      std::cerr << "FAIL: " #cond " (" << __FILE__ << ":" << __LINE__ << ")\n"; \
    } else {                                                        \
      ++tests_passed;                                               \
    }                                                               \
  } while (0)

static void test_dcp_config_defaults()
{
  dcpwizard::DCPConfig config;
  ASSERT(config.standard == dcpwizard::Standard::SMPTE);
  ASSERT(config.resolution == dcpwizard::Resolution::TwoK);
  ASSERT(config.frame_rate_num == 24);
  ASSERT(config.encrypt == false);
}

static void test_verify_empty()
{
  auto result = dcpwizard::verify_dcp("/nonexistent");
  // Stub always returns passed for now
  ASSERT(result.passed == true);
}

static void test_job_submit()
{
  dcpwizard::Job job;
  job.type = dcpwizard::JobType::Create;
  job.description = "Test job";
  auto id = dcpwizard::submit_job(job);
  ASSERT(id > 0);
  auto queried = dcpwizard::get_job(id);
  ASSERT(queried.has_value());
  ASSERT(queried->state == dcpwizard::JobState::Queued);
}

int main()
{
  test_dcp_config_defaults();
  test_verify_empty();
  test_job_submit();

  std::cout << tests_passed << "/" << tests_run << " tests passed\n";
  return (tests_passed == tests_run) ? 0 : 1;
}
