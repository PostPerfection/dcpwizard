#pragma once

#include <cstdint>
#include <string>

namespace dcpwizard
{

/// Start the REST API server for headless/batch operation.
int start_rest_api(uint16_t port = 8080, const std::string& bind_addr = "127.0.0.1");

} // namespace dcpwizard
