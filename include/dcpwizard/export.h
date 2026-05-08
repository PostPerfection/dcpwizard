#pragma once

#include <filesystem>
#include <string>

namespace dcpwizard
{

enum class ExportFormat
{
  ProRes,
  H264,
  H265,
  DNxHR,
  ImageSequence
};

struct ExportConfig
{
  std::filesystem::path dcp_dir;
  std::filesystem::path output_file;   // or output_dir for image sequences
  ExportFormat format = ExportFormat::ProRes;
  std::string quality = "hq";          // proxy, lt, standard, hq, 4444
  uint32_t start_frame = 0;
  uint32_t end_frame = 0;             // 0 = all
};

/// Export/extract frames from a DCP to another format.
int export_dcp(const ExportConfig& config);

/// Extract a single frame as an image file (for thumbnails/preview).
int extract_frame(const std::filesystem::path& dcp_dir,
                  uint64_t frame_number,
                  const std::filesystem::path& output_image);

} // namespace dcpwizard
