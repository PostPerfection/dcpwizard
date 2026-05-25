#include "dcpwizard/cpl.h"

#include <chrono>
#include <filesystem>
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

int generate_cpl(const CPLConfig& config, const std::filesystem::path& output_file)
{
  std::ofstream out(output_file);
  if (!out)
  {
    spdlog::error("Cannot open CPL output file: {}", output_file.string());
    return 1;
  }

  std::string ns = "http://www.smpte-ra.org/schemas/429-7/2006/CPL";

  out << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
  out << "<CompositionPlaylist xmlns=\"" << ns << "\">\n";
  out << "  <Id>urn:uuid:" << config.id << "</Id>\n";
  out << "  <AnnotationText>" << config.title << "</AnnotationText>\n";
  out << "  <IssueDate>" << now_iso8601() << "</IssueDate>\n";
  out << "  <Issuer>dcpwizard</Issuer>\n";
  out << "  <Creator>DCP Wizard 0.1.0</Creator>\n";
  out << "  <ContentTitleText>" << config.title << "</ContentTitleText>\n";
  out << "  <ContentKind>" << config.content_kind << "</ContentKind>\n";
  out << "  <ContentVersion>\n";
  out << "    <Id>urn:uri:" << config.id << "_" << now_iso8601() << "</Id>\n";
  out << "    <LabelText>" << config.title << "_" << now_iso8601() << "</LabelText>\n";
  out << "  </ContentVersion>\n";
  out << "  <RatingList/>\n";
  out << "  <ReelList>\n";

  for (const auto& reel : config.reels)
  {
    out << "    <Reel>\n";
    out << "      <Id>urn:uuid:" << reel.id << "</Id>\n";
    out << "      <AssetList>\n";

    // Picture
    out << "        <MainPicture>\n";
    out << "          <Id>urn:uuid:" << reel.picture.id << "</Id>\n";
    out << "          <EditRate>" << reel.picture.frame_rate_num << " "
        << reel.picture.frame_rate_den << "</EditRate>\n";
    out << "          <IntrinsicDuration>" << reel.picture.duration << "</IntrinsicDuration>\n";
    out << "          <EntryPoint>" << reel.picture.entry_point << "</EntryPoint>\n";
    out << "          <Duration>" << reel.picture.duration << "</Duration>\n";
    out << "          <FrameRate>" << reel.picture.frame_rate_num << " "
        << reel.picture.frame_rate_den << "</FrameRate>\n";
    out << "          <ScreenAspectRatio>1998 1080</ScreenAspectRatio>\n";
    out << "        </MainPicture>\n";

    // Sound (optional)
    if (!reel.sound.asset_id.empty())
    {
      out << "        <MainSound>\n";
      out << "          <Id>urn:uuid:" << reel.sound.id << "</Id>\n";
      out << "          <EditRate>" << reel.sound.frame_rate_num << " "
          << reel.sound.frame_rate_den << "</EditRate>\n";
      out << "          <IntrinsicDuration>" << reel.sound.duration << "</IntrinsicDuration>\n";
      out << "          <EntryPoint>" << reel.sound.entry_point << "</EntryPoint>\n";
      out << "          <Duration>" << reel.sound.duration << "</Duration>\n";
      out << "        </MainSound>\n";
    }

    out << "      </AssetList>\n";
    out << "    </Reel>\n";
  }

  out << "  </ReelList>\n";
  out << "</CompositionPlaylist>\n";

  spdlog::info("Generated CPL: {}", output_file.string());
  return 0;
}

} // namespace dcpwizard
