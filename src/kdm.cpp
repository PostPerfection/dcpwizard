#include "dcpwizard/kdm.h"

#include <KM_util.h>
#include <chrono>
#include <ctime>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <spdlog/spdlog.h>
#include <sstream>

namespace fs = std::filesystem;

namespace dcpwizard
{

static std::string make_uuid()
{
  Kumu::UUID uuid;
  Kumu::GenRandomValue(uuid);
  char buf[64];
  uuid.EncodeString(buf, sizeof(buf));
  return buf;
}

static std::string time_to_iso(std::chrono::system_clock::time_point tp)
{
  auto t = std::chrono::system_clock::to_time_t(tp);
  struct tm tm_buf;
  gmtime_r(&t, &tm_buf);
  std::ostringstream ss;
  ss << std::put_time(&tm_buf, "%Y-%m-%dT%H:%M:%S+00:00");
  return ss.str();
}

int generate_kdm(const KDMConfig& config)
{
  if (!fs::exists(config.dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", config.dcp_dir.string());
    return 1;
  }
  if (!fs::exists(config.certificate))
  {
    spdlog::error("Certificate not found: {}", config.certificate.string());
    return 1;
  }

  spdlog::info("Generating KDM for: {}", config.content_title);

  std::string kdm_id = make_uuid();
  auto not_before = time_to_iso(config.not_valid_before);
  auto not_after = time_to_iso(config.not_valid_after);

  // Generate SMPTE 430-1 KDM XML
  std::ostringstream xml;
  xml << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
  xml << "<DCinemaSecurityMessage xmlns=\"http://www.smpte-ra.org/schemas/430-3/2006/ETM\">\n";
  xml << "  <AuthenticatedPublic Id=\"ID_AuthenticatedPublic\">\n";
  xml << "    <MessageId>urn:uuid:" << kdm_id << "</MessageId>\n";
  xml << "    <MessageType>http://www.smpte-ra.org/430-1/2006/KDM#kdm-key-type</MessageType>\n";
  xml << "    <AnnotationText>" << config.content_title << "</AnnotationText>\n";
  xml << "    <IssueDate>" << time_to_iso(std::chrono::system_clock::now()) << "</IssueDate>\n";
  xml << "    <RequiredExtensions>\n";
  xml << "      <KDMRequiredExtensions xmlns=\"http://www.smpte-ra.org/schemas/430-1/2006/KDM\">\n";
  xml << "        <ContentTitleText>" << config.content_title << "</ContentTitleText>\n";
  xml << "        <ContentKeysNotValidBefore>" << not_before << "</ContentKeysNotValidBefore>\n";
  xml << "        <ContentKeysNotValidAfter>" << not_after << "</ContentKeysNotValidAfter>\n";
  xml << "      </KDMRequiredExtensions>\n";
  xml << "    </RequiredExtensions>\n";
  xml << "  </AuthenticatedPublic>\n";
  xml << "</DCinemaSecurityMessage>\n";

  auto output = config.output_file;
  if (output.empty())
    output = config.dcp_dir / ("KDM_" + kdm_id + ".xml");

  std::ofstream f(output);
  f << xml.str();
  spdlog::info("KDM written: {}", output.string());
  return 0;
}

int generate_kdm_batch(const fs::path& dcp_dir,
                       const std::vector<fs::path>& certificates,
                       const fs::path& output_dir)
{
  if (!fs::exists(dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", dcp_dir.string());
    return 1;
  }

  fs::create_directories(output_dir);
  spdlog::info("Batch KDM generation: {} recipients", certificates.size());

  int failures = 0;
  for (const auto& cert : certificates)
  {
    KDMConfig config;
    config.dcp_dir = dcp_dir;
    config.certificate = cert;
    config.content_title = dcp_dir.filename().string();
    config.not_valid_before = std::chrono::system_clock::now();
    config.not_valid_after = std::chrono::system_clock::now() + std::chrono::hours(24 * 30);
    config.output_file = output_dir / ("KDM_" + cert.stem().string() + ".xml");

    if (generate_kdm(config) != 0)
      ++failures;
  }

  return failures > 0 ? 1 : 0;
}

} // namespace dcpwizard
