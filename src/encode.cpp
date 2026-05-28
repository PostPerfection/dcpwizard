#include "dcpwizard/encode.h"

#include <array>
#include <cstdio>
#include <filesystem>
#include <spdlog/spdlog.h>
#include <stdexcept>
#include <string>

namespace fs = std::filesystem;

namespace dcpwizard
{

int encode_j2k(const EncodeConfig& config)
{
  if (!fs::exists(config.input_dir) || !fs::is_directory(config.input_dir))
  {
    spdlog::error("Input directory does not exist: {}", config.input_dir.string());
    return 1;
  }

  fs::create_directories(config.output_dir);

  // Find grk_compress: check PATH, then common locations
  std::string grk_compress = "grk_compress";

  // Build command: batch encode with DCI cinema2K profile at 24fps
  // -w 24         : DCI 2K cinema profile, 24fps
  // -batch_src    : input directory
  // -a            : output directory
  // -O J2K        : output raw J2K codestreams
  std::string cmd = grk_compress;
  cmd += " -w 24";
  cmd += " -batch_src " + config.input_dir.string();
  cmd += " -a " + config.output_dir.string();
  cmd += " -O J2K";

  if (config.threads > 0)
    cmd += " -H " + std::to_string(config.threads);

  spdlog::info("Encoding J2K: {} → {}", config.input_dir.string(),
               config.output_dir.string());
  spdlog::debug("Command: {}", cmd);

  int rc = std::system(cmd.c_str());
  if (rc != 0)
  {
    spdlog::error("grk_compress failed with exit code {}", rc);
    return 1;
  }

  // Count output files
  uint32_t count = 0;
  for (const auto& entry : fs::directory_iterator(config.output_dir))
  {
    if (entry.path().extension() == ".j2k" || entry.path().extension() == ".j2c")
      ++count;
  }
  spdlog::info("Encoded {} frames", count);
  return 0;
}

} // namespace dcpwizard
