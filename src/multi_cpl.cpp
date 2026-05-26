#include "dcpwizard/multi_cpl.h"

#include <filesystem>
#include <fstream>
#include <regex>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

std::vector<std::string> list_cpls(const fs::path& dcp_dir)
{
  std::vector<std::string> cpls;

  if (!fs::exists(dcp_dir))
    return cpls;

  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    if (entry.path().extension() != ".xml")
      continue;

    std::ifstream f(entry.path());
    std::string content((std::istreambuf_iterator<char>(f)),
                        std::istreambuf_iterator<char>());

    if (content.find("CompositionPlaylist") != std::string::npos)
    {
      // Extract UUID from <Id> element
      std::regex id_re("<Id>urn:uuid:([^<]+)</Id>");
      std::smatch m;
      if (std::regex_search(content, m, id_re))
        cpls.push_back(m[1].str());
    }
  }

  spdlog::info("Found {} CPLs in {}", cpls.size(), dcp_dir.string());
  return cpls;
}

std::vector<TimelineEntry> get_timeline(const fs::path& dcp_dir,
                                        const std::string& cpl_id)
{
  std::vector<TimelineEntry> timeline;

  if (!fs::exists(dcp_dir))
    return timeline;

  // Find CPL with matching ID
  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    if (entry.path().extension() != ".xml")
      continue;

    std::ifstream f(entry.path());
    std::string content((std::istreambuf_iterator<char>(f)),
                        std::istreambuf_iterator<char>());

    if (content.find(cpl_id) == std::string::npos)
      continue;
    if (content.find("CompositionPlaylist") == std::string::npos)
      continue;

    // Parse reels
    std::regex reel_re("<Reel>([\\s\\S]*?)</Reel>");
    auto reels_begin = std::sregex_iterator(content.begin(), content.end(), reel_re);
    auto reels_end = std::sregex_iterator();

    uint64_t frame_pos = 0;
    for (auto it = reels_begin; it != reels_end; ++it)
    {
      std::string reel_xml = (*it)[1].str();

      // Extract duration
      std::regex dur_re("<Duration>(\\d+)</Duration>");
      std::smatch dm;
      uint64_t duration = 0;
      if (std::regex_search(reel_xml, dm, dur_re))
        duration = std::stoull(dm[1].str());

      // Extract reel ID
      std::regex id_re("<Id>urn:uuid:([^<]+)</Id>");
      std::smatch im;
      std::string reel_id;
      if (std::regex_search(reel_xml, im, id_re))
        reel_id = im[1].str();

      if (reel_xml.find("MainPicture") != std::string::npos)
      {
        TimelineEntry e;
        e.reel_id = reel_id;
        e.start_frame = frame_pos;
        e.end_frame = frame_pos + duration;
        e.asset_type = "picture";
        timeline.push_back(e);
      }
      if (reel_xml.find("MainSound") != std::string::npos)
      {
        TimelineEntry e;
        e.reel_id = reel_id;
        e.start_frame = frame_pos;
        e.end_frame = frame_pos + duration;
        e.asset_type = "sound";
        timeline.push_back(e);
      }

      frame_pos += duration;
    }
    break;
  }

  spdlog::info("Timeline for CPL {}: {} entries", cpl_id, timeline.size());
  return timeline;
}

int create_multi_cpl(const MultiCPLConfig& config)
{
  if (!fs::exists(config.dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", config.dcp_dir.string());
    return 1;
  }

  spdlog::info("Multi-CPL DCP with {} compositions", config.cpl_ids.size());
  // Multi-CPL packages share the same reel assets but have different CPLs
  // This is handled by the ASSETMAP referencing all CPLs
  auto existing_cpls = list_cpls(config.dcp_dir);
  spdlog::info("Existing CPLs: {}, requested: {}", existing_cpls.size(), config.cpl_ids.size());

  return 0;
}

} // namespace dcpwizard
