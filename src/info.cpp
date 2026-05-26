#include "dcpwizard/info.h"

#include <filesystem>
#include <fstream>
#include <regex>
#include <spdlog/spdlog.h>
#include <string>

namespace fs = std::filesystem;

namespace dcpwizard
{

static std::string extract_xml_value(const std::string& xml, const std::string& tag)
{
  auto open = xml.find("<" + tag + ">");
  if (open == std::string::npos) return {};
  auto start = open + tag.size() + 2;
  auto close = xml.find("</" + tag + ">", start);
  if (close == std::string::npos) return {};
  return xml.substr(start, close - start);
}

DCPInfo inspect_dcp(const fs::path& dcp_dir)
{
  spdlog::info("Inspecting DCP: {}", dcp_dir.string());
  DCPInfo info;

  if (!fs::exists(dcp_dir))
    return info;

  // Find and parse CPL
  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    auto name = entry.path().filename().string();
    if (entry.path().extension() != ".xml")
      continue;

    std::ifstream f(entry.path());
    std::string content((std::istreambuf_iterator<char>(f)),
                        std::istreambuf_iterator<char>());

    if (content.find("CompositionPlaylist") != std::string::npos)
    {
      info.title = extract_xml_value(content, "ContentTitleText");
      if (info.title.empty())
        info.title = extract_xml_value(content, "AnnotationText");

      auto kind = extract_xml_value(content, "ContentKind");
      if (!kind.empty())
        info.standard = kind;

      // Detect SMPTE vs Interop
      if (content.find("smpte-ra.org") != std::string::npos)
        info.standard = "SMPTE";
      else if (content.find("digicine.com") != std::string::npos)
        info.standard = "Interop";

      // Parse edit rate
      auto rate = extract_xml_value(content, "EditRate");
      if (!rate.empty())
        info.frame_rate = rate;

      // Count reels
      size_t pos = 0;
      while ((pos = content.find("<Reel>", pos)) != std::string::npos)
      {
        ++info.reel_count;
        ++pos;
      }

      // Parse duration
      auto dur = extract_xml_value(content, "Duration");
      if (!dur.empty())
      {
        try { info.duration_frames = std::stoull(dur); } catch (...) {}
      }

      // Detect encryption
      if (content.find("KeyId") != std::string::npos)
        info.encrypted = true;

      // Detect stereo 3D
      if (content.find("Stereoscopic") != std::string::npos ||
          content.find("MainStereoscopicPicture") != std::string::npos)
        info.stereo_3d = true;
    }
  }

  // Calculate total size
  for (const auto& entry : fs::recursive_directory_iterator(dcp_dir))
  {
    if (entry.is_regular_file())
      info.total_size_bytes += fs::file_size(entry.path());
  }

  // Detect resolution from picture MXF filename or size heuristic
  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    if (entry.path().extension() == ".mxf")
    {
      auto size = fs::file_size(entry.path());
      if (size > 1024 * 1024 * 100) // > 100MB likely picture
        info.resolution = (size > 1024ULL * 1024 * 1024 * 4) ? "4096x2160" : "2048x1080";
    }
  }

  spdlog::info("  Title: {}", info.title);
  spdlog::info("  Standard: {}", info.standard);
  spdlog::info("  Reels: {}", info.reel_count);
  spdlog::info("  Duration: {} frames", info.duration_frames);
  spdlog::info("  Total size: {} bytes", info.total_size_bytes);

  return info;
}

} // namespace dcpwizard
