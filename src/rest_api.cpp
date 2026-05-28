#include "dcpwizard/rest_api.h"
#include "dcpwizard/job_queue.h"

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment(lib, "Ws2_32.lib")
using ssize_t = int;
#define CLOSE_SOCKET closesocket
#else
#include <arpa/inet.h>
#include <cstring>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
#define CLOSE_SOCKET close
#endif

#include <spdlog/spdlog.h>
#include <string>

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
  ssize_t n = recv(client_fd, buf, sizeof(buf) - 1, 0);
  if (n <= 0)
  {
    CLOSE_SOCKET(client_fd);
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

  send(client_fd, response.c_str(), static_cast<int>(response.size()), 0);
  CLOSE_SOCKET(client_fd);
}

int start_rest_api(uint16_t port, const std::string& bind_addr)
{
  spdlog::info("Starting REST API on {}:{}", bind_addr, port);

#ifdef _WIN32
  WSADATA wsa_data;
  if (WSAStartup(MAKEWORD(2, 2), &wsa_data) != 0)
  {
    spdlog::error("WSAStartup failed");
    return 1;
  }
#endif

  int server_fd = static_cast<int>(socket(AF_INET, SOCK_STREAM, 0));
  if (server_fd < 0)
  {
    spdlog::error("Failed to create socket");
    return 1;
  }

  int opt = 1;
  setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, reinterpret_cast<const char*>(&opt), sizeof(opt));

  struct sockaddr_in addr{};
  addr.sin_family = AF_INET;
  addr.sin_port = htons(port);
  inet_pton(AF_INET, bind_addr.c_str(), &addr.sin_addr);

  if (bind(server_fd, reinterpret_cast<struct sockaddr*>(&addr), sizeof(addr)) < 0)
  {
    spdlog::error("Failed to bind to {}:{}", bind_addr, port);
    CLOSE_SOCKET(server_fd);
    return 1;
  }

  if (listen(server_fd, 16) < 0)
  {
    spdlog::error("Failed to listen");
    CLOSE_SOCKET(server_fd);
    return 1;
  }

  spdlog::info("REST API listening on {}:{}", bind_addr, port);

  while (true)
  {
    int client_fd = static_cast<int>(accept(server_fd, nullptr, nullptr));
    if (client_fd < 0)
      continue;
    handle_client(client_fd);
  }

  CLOSE_SOCKET(server_fd);
#ifdef _WIN32
  WSACleanup();
#endif
  return 0;
}

} // namespace dcpwizard
