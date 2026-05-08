#include "dcpwizard/rest_api.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int start_rest_api(uint16_t port, const std::string& bind_addr)
{
  spdlog::info("Starting REST API on {}:{}", bind_addr, port);
  // TODO: implement HTTP server for batch/headless operation
  return 0;
}

} // namespace dcpwizard
