#include "dcpwizard/pkl.h"

#include <chrono>
#include <fstream>
#include <spdlog/spdlog.h>

namespace dcpwizard
{

static std::string now_iso8601()
{
  auto now = std::chrono::system_clock::now();
  auto time = std::chrono::system_clock::to_time_t(now);
  char buf[64];
  std::strftime(buf, sizeof(buf), "%Y-%m-%dT%H:%M:%S+00:00", std::gmtime(&time));
  return buf;
}

int generate_pkl(const PKLConfig& config,
                 const std::filesystem::path& output_file)
{
  std::ofstream out(output_file);
  if (!out)
  {
    spdlog::error("Cannot open PKL output file: {}", output_file.string());
    return 1;
  }

  std::string ns = "http://www.smpte-ra.org/schemas/429-8/2007/PKL";

  out << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
  out << "<PackingList xmlns=\"" << ns << "\">\n";
  out << "  <Id>urn:uuid:" << config.id << "</Id>\n";
  out << "  <AnnotationText>" << config.annotation << "</AnnotationText>\n";
  out << "  <IssueDate>" << now_iso8601() << "</IssueDate>\n";
  out << "  <Issuer>" << config.issuer << "</Issuer>\n";
  out << "  <Creator>" << config.creator << "</Creator>\n";
  out << "  <AssetList>\n";

  for (const auto& entry : config.entries)
  {
    out << "    <Asset>\n";
    out << "      <Id>urn:uuid:" << entry.id << "</Id>\n";
    out << "      <AnnotationText>" << entry.original_filename << "</AnnotationText>\n";
    out << "      <Hash>" << entry.hash << "</Hash>\n";
    out << "      <Size>" << entry.size << "</Size>\n";
    out << "      <Type>" << entry.type << "</Type>\n";
    out << "    </Asset>\n";
  }

  out << "  </AssetList>\n";
  out << "</PackingList>\n";

  spdlog::info("Generated PKL: {}", output_file.string());
  return 0;
}

} // namespace dcpwizard
