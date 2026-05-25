#include "dcpwizard/assetmap.h"

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

int generate_assetmap(const AssetMapConfig& config,
                      const std::filesystem::path& output_dir)
{
  // VOLINDEX
  {
    auto volindex_path = output_dir / "VOLINDEX.xml";
    std::ofstream out(volindex_path);
    if (!out)
    {
      spdlog::error("Cannot write VOLINDEX: {}", volindex_path.string());
      return 1;
    }
    out << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    out << "<VolumeIndex xmlns=\"http://www.smpte-ra.org/schemas/429-9/2007/AM\">\n";
    out << "  <Index>1</Index>\n";
    out << "</VolumeIndex>\n";
  }

  // ASSETMAP
  {
    auto assetmap_path = output_dir / "ASSETMAP.xml";
    std::ofstream out(assetmap_path);
    if (!out)
    {
      spdlog::error("Cannot write ASSETMAP: {}", assetmap_path.string());
      return 1;
    }

    std::string ns = "http://www.smpte-ra.org/schemas/429-9/2007/AM";

    out << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    out << "<AssetMap xmlns=\"" << ns << "\">\n";
    out << "  <Id>urn:uuid:" << config.id << "</Id>\n";
    out << "  <AnnotationText>" << config.annotation << "</AnnotationText>\n";
    out << "  <VolumeCount>1</VolumeCount>\n";
    out << "  <IssueDate>" << now_iso8601() << "</IssueDate>\n";
    out << "  <Issuer>" << config.issuer << "</Issuer>\n";
    out << "  <Creator>" << config.creator << "</Creator>\n";
    out << "  <AssetList>\n";

    for (const auto& entry : config.entries)
    {
      out << "    <Asset>\n";
      out << "      <Id>urn:uuid:" << entry.id << "</Id>\n";
      out << "      <ChunkList>\n";
      out << "        <Chunk>\n";
      out << "          <Path>" << entry.path << "</Path>\n";
      if (entry.size > 0)
        out << "          <VolumeIndex>1</VolumeIndex>\n";
      out << "        </Chunk>\n";
      out << "      </ChunkList>\n";
      out << "    </Asset>\n";
    }

    out << "  </AssetList>\n";
    out << "</AssetMap>\n";
  }

  spdlog::info("Generated ASSETMAP and VOLINDEX in {}", output_dir.string());
  return 0;
}

} // namespace dcpwizard
