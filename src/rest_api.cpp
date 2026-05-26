#include "dcpwizard/rest_api.h"
#include "dcpwizard/job_queue.h"

#include <arpa/inet.h>
#include <cstring>
#include <netinet/in.h>
#include <spdlog/spdlog.h>
#include <string>
#include <sys/socket.h>
#include <unistd.h>

namespace dcpwizard
{

static std::string build_json_response(int code, const std::string& body)
{
  std::string response = "HTTP/1.1 " + std::to_string(code) + " OK\r\n";
  response += "Content-Type: application/json\r\n";
  response += "Content-Length: " + std::to_string(body.size()) + "\r\n";
  response += "Connection: close\r\n\r\n";
  response += body;
  return response;
}

static void handle_client(int client_fd)
{
  char buf[4096];
  ssize_t n = read(client_fd, buf, sizeof(buf) - 1);
  if (n <= 0)
  {
    close(client_fd);
    return;
  }
  buf[n] = '\0';
  std::string request(buf);

  // Simple router
  std::string response;
  if (request.find("GET /health") != std::string::npos)
  {
    response = build_json_response(200, R"({"status":"ok"})");
  }
  else if (request.find("GET /jobs") != std::string::npos)
  {
    auto jobs = list_jobs();
    std::string body = "[";
    for (size_t i = 0; i < jobs.size(); ++i)
    {
      if (i > 0) body += ",";
      body += "{\"id\":" + std::to_string(jobs[i].id) +
              ",\"type\":\"" + job_type_to_string(jobs[i].type) +
              "\",\"state\":\"" + job_state_to_string(jobs[i].state) +
              "\",\"progress\":" + std::to_string(jobs[i].progress) + "}";
    }
    body += "]";
    response = build_json_response(200, body);
  }
  else
  {
    response = build_json_response(404, R"({"error":"not found"})");
  }

  write(client_fd, response.c_str(), response.size());
  close(client_fd);
}

int start_rest_api(uint16_t port, const std::string& bind_addr)
{
  spdlog::info("Starting REST API on {}:{}", bind_addr, port);

  int server_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (server_fd < 0)
  {
    spdlog::error("Failed to create socket");
    return 1;
  }

  int opt = 1;
  setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

  struct sockaddr_in addr{};
  addr.sin_family = AF_INET;
  addr.sin_port = htons(port);
  inet_pton(AF_INET, bind_addr.c_str(), &addr.sin_addr);

  if (bind(server_fd, reinterpret_cast<struct sockaddr*>(&addr), sizeof(addr)) < 0)
  {
    spdlog::error("Failed to bind to {}:{}", bind_addr, port);
    close(server_fd);
    return 1;
  }

  if (listen(server_fd, 16) < 0)
  {
    spdlog::error("Failed to listen");
    close(server_fd);
    return 1;
  }

  spdlog::info("REST API listening on {}:{}", bind_addr, port);

  while (true)
  {
    int client_fd = accept(server_fd, nullptr, nullptr);
    if (client_fd < 0)
      continue;
    handle_client(client_fd);
  }

  close(server_fd);
  return 0;
}

} // namespace dcpwizard
