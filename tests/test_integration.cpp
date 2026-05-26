#include <cstdlib>
#include <filesystem>
#include <iostream>

#include "dcpwizard/dcp.h"
#include "dcpwizard/info.h"

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

static void test_create_dcp_stub()
{
  dcpwizard::DCPConfig config;
  config.title = "Integration Test";
  config.output_dir = "/tmp/dcpwizard_test_output";
  int rc = dcpwizard::create_dcp(config);
  // No video_dir set → should fail gracefully
  ASSERT(rc != 0);
}

static void test_inspect_nonexistent()
{
  auto info = dcpwizard::inspect_dcp("/nonexistent");
  ASSERT(info.title.empty());
}

int main()
{
  test_create_dcp_stub();
  test_inspect_nonexistent();

  std::cout << tests_passed << "/" << tests_run << " tests passed\n";
  return (tests_passed == tests_run) ? 0 : 1;
}
