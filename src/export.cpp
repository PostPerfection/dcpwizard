#include "dcpwizard/export.h"

#include <filesystem>
#include <spdlog/spdlog.h>
#include <string>

namespace fs = std::filesystem;

namespace dcpwizard
{

static std::string format_to_codec(ExportFormat fmt, const std::string& quality)
{
  switch (fmt)
  {
    case ExportFormat::ProRes:
    {
      int profile = 3; // HQ
      if (quality == "proxy") profile = 0;
      else if (quality == "lt") profile = 1;
      else if (quality == "standard") profile = 2;
      else if (quality == "4444") profile = 4;
      return "-c:v prores_ks -profile:v " + std::to_string(profile);
    }
    case ExportFormat::H264:
      return "-c:v libx264 -crf 18 -preset slow";
    case ExportFormat::H265:
      return "-c:v libx265 -crf 20 -preset slow";
    case ExportFormat::DNxHR:
      return "-c:v dnxhd -profile:v dnxhr_hq";
    default:
      return "";
  }
}

int export_dcp(const ExportConfig& config)
{
  if (!fs::exists(config.dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", config.dcp_dir.string());
    return 1;
  }

  spdlog::info("Exporting DCP: {} → {}", config.dcp_dir.string(),
               config.output_file.string());

  // Find picture MXF
  fs::path picture_mxf;
  for (const auto& entry : fs::directory_iterator(config.dcp_dir))
  {
    if (entry.path().extension() == ".mxf" &&
        entry.path().filename().string().find("audio") == std::string::npos)
    {
      picture_mxf = entry.path();
      break;
    }
  }

  if (picture_mxf.empty())
  {
    spdlog::error("No picture MXF found in DCP");
    return 1;
  }

  if (config.format == ExportFormat::ImageSequence)
  {
    // Extract frames using asdcp-unwrap or ffmpeg
    fs::create_directories(config.output_file);
    std::string cmd = "ffmpeg -y -i \"" + picture_mxf.string() +
                      "\" -pix_fmt rgb48le \"" +
                      (config.output_file / "frame_%06d.tiff").string() +
                      "\" 2>/dev/null";
    return system(cmd.c_str()) == 0 ? 0 : 1;
  }

  // Transcode to target format
  auto codec = format_to_codec(config.format, config.quality);
  std::string cmd = "ffmpeg -y -i \"" + picture_mxf.string() + "\"";

  // Add audio if present
  for (const auto& entry : fs::directory_iterator(config.dcp_dir))
  {
    if (entry.path().extension() == ".mxf" &&
        entry.path().filename().string().find("audio") != std::string::npos)
    {
      cmd += " -i \"" + entry.path().string() + "\"";
      break;
    }
  }

  cmd += " " + codec + " \"" + config.output_file.string() + "\" 2>/dev/null";

  spdlog::debug("Export command: {}", cmd);
  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    spdlog::error("Export failed with code {}", rc);
    return 1;
  }

  spdlog::info("Export complete: {}", config.output_file.string());
  return 0;
}

int extract_frame(const fs::path& dcp_dir,
                  uint64_t frame_number,
                  const fs::path& output_image)
{
  if (!fs::exists(dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", dcp_dir.string());
    return 1;
  }

  // Find picture MXF
  fs::path picture_mxf;
  for (const auto& entry : fs::directory_iterator(dcp_dir))
  {
    if (entry.path().extension() == ".mxf" &&
        entry.path().filename().string().find("audio") == std::string::npos)
    {
      picture_mxf = entry.path();
      break;
    }
  }

  if (picture_mxf.empty())
  {
    spdlog::error("No picture MXF found");
    return 1;
  }

  // Extract specific frame using ffmpeg
  std::string cmd = "ffmpeg -y -i \"" + picture_mxf.string() +
                    "\" -vf \"select=eq(n\\," + std::to_string(frame_number) +
                    ")\" -frames:v 1 -pix_fmt rgb48le \"" +
                    output_image.string() + "\" 2>/dev/null";

  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    spdlog::error("Frame extraction failed");
    return 1;
  }

  spdlog::info("Extracted frame {} → {}", frame_number, output_image.string());
  return 0;
}

} // namespace dcpwizard
