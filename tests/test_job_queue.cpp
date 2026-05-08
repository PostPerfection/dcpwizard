#include <cstdlib>
#include <iostream>

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

static void test_submit_and_cancel()
{
  dcpwizard::Job job;
  job.type = dcpwizard::JobType::Encode;
  job.description = "Encode test";
  auto id = dcpwizard::submit_job(job);
  ASSERT(id > 0);
  bool cancelled = dcpwizard::cancel_job(id);
  ASSERT(cancelled);
  auto queried = dcpwizard::get_job(id);
  ASSERT(queried.has_value());
  ASSERT(queried->state == dcpwizard::JobState::Cancelled);
}

static void test_list_jobs()
{
  auto jobs = dcpwizard::list_jobs();
  ASSERT(!jobs.empty());
}

int main()
{
  test_submit_and_cancel();
  test_list_jobs();

  std::cout << tests_passed << "/" << tests_run << " tests passed\n";
  return (tests_passed == tests_run) ? 0 : 1;
}
