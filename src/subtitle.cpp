#include "dcpwizard/subtitle.h"

#include <filesystem>
#include <fstream>
#include <regex>
#include <spdlog/spdlog.h>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

namespace dcpwizard
{

struct SubEntry
{
  double start_sec = 0;
  double end_sec = 0;
  std::string text;
};

static std::vector<SubEntry> parse_srt(const fs::path& file)
{
  std::vector<SubEntry> entries;
  std::ifstream f(file);
  std::string line;
  SubEntry current;
  int state = 0; // 0=index, 1=time, 2=text

  while (std::getline(f, line))
  {
    // Remove \r
    if (!line.empty() && line.back() == '\r')
      line.pop_back();

    if (state == 0)
    {
      if (!line.empty() && std::isdigit(line[0]))
        state = 1;
    }
    else if (state == 1)
    {
      // Parse time: 00:01:23,456 --> 00:01:25,789
      std::regex time_re(R"((\d+):(\d+):(\d+),(\d+)\s*-->\s*(\d+):(\d+):(\d+),(\d+))");
      std::smatch m;
      if (std::regex_search(line, m, time_re))
      {
        current.start_sec = std::stoi(m[1]) * 3600.0 + std::stoi(m[2]) * 60.0 +
                            std::stoi(m[3]) + std::stoi(m[4]) / 1000.0;
        current.end_sec = std::stoi(m[5]) * 3600.0 + std::stoi(m[6]) * 60.0 +
                          std::stoi(m[7]) + std::stoi(m[8]) / 1000.0;
      }
      state = 2;
      current.text.clear();
    }
    else if (state == 2)
    {
      if (line.empty())
      {
        if (!current.text.empty())
          entries.push_back(current);
        current = SubEntry{};
        state = 0;
      }
      else
      {
        if (!current.text.empty())
          current.text += "\n";
        current.text += line;
      }
    }
  }
  if (!current.text.empty())
    entries.push_back(current);

  return entries;
}

int import_subtitles(const SubtitleConfig& config)
{
  if (!fs::exists(config.input_file))
  {
    spdlog::error("Subtitle file not found: {}", config.input_file.string());
    return 1;
  }

  spdlog::info("Importing subtitles: {}", config.input_file.string());

  auto entries = parse_srt(config.input_file);
  if (entries.empty())
  {
    spdlog::error("No subtitle entries found");
    return 1;
  }

  // Generate SMPTE ST 428-7 XML subtitles
  auto output = config.input_file.parent_path() /
                (config.input_file.stem().string() + "_smpte.xml");

  std::ofstream f(output);
  f << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
  f << "<SubtitleReel xmlns=\"http://www.smpte-ra.org/schemas/428-7/2010/DCST\">\n";
  f << "  <Id>urn:uuid:00000000-0000-0000-0000-000000000000</Id>\n";
  f << "  <Language>" << (config.language.empty() ? "en" : config.language) << "</Language>\n";
  f << "  <EditRate>24 1</EditRate>\n";
  f << "  <TimeCodeRate>24</TimeCodeRate>\n";
  f << "  <LoadFont ID=\"font1\">" << (config.font_family.empty() ? "Arial" : config.font_family) << "</LoadFont>\n";
  f << "  <SubtitleList>\n";
  f << "    <Font ID=\"font1\" Size=\"" << static_cast<int>(config.font_size) << "\">\n";

  int idx = 0;
  for (const auto& e : entries)
  {
    int sh = static_cast<int>(e.start_sec) / 3600;
    int sm = (static_cast<int>(e.start_sec) % 3600) / 60;
    int ss = static_cast<int>(e.start_sec) % 60;
    int sf = static_cast<int>((e.start_sec - static_cast<int>(e.start_sec)) * 24);

    int eh = static_cast<int>(e.end_sec) / 3600;
    int em = (static_cast<int>(e.end_sec) % 3600) / 60;
    int es = static_cast<int>(e.end_sec) % 60;
    int ef = static_cast<int>((e.end_sec - static_cast<int>(e.end_sec)) * 24);

    char tc_in[32], tc_out[32];
    snprintf(tc_in, sizeof(tc_in), "%02d:%02d:%02d:%02d", sh, sm, ss, sf);
    snprintf(tc_out, sizeof(tc_out), "%02d:%02d:%02d:%02d", eh, em, es, ef);

    f << "      <Subtitle SpotNumber=\"" << ++idx << "\" TimeIn=\"" << tc_in
      << "\" TimeOut=\"" << tc_out << "\">\n";
    f << "        <Text VPosition=\"10\" VAlign=\"bottom\">" << e.text << "</Text>\n";
    f << "      </Subtitle>\n";
  }

  f << "    </Font>\n";
  f << "  </SubtitleList>\n";
  f << "</SubtitleReel>\n";

  spdlog::info("Converted {} subtitles to SMPTE XML: {}", entries.size(), output.string());
  return 0;
}

int burnin_subtitles(const fs::path& video_dir,
                     const fs::path& subtitle_file,
                     const fs::path& output_dir)
{
  if (!fs::exists(video_dir) || !fs::exists(subtitle_file))
  {
    spdlog::error("Input not found");
    return 1;
  }

  fs::create_directories(output_dir);
  spdlog::info("Burning subtitles into frames: {} + {}", video_dir.string(), subtitle_file.string());

  // Use ffmpeg to overlay subtitles on image sequence
  std::string cmd = "ffmpeg -y -framerate 24 -i \"" + (video_dir / "frame_%06d.tiff").string() +
                    "\" -vf subtitles=\"" + subtitle_file.string() +
                    "\" -pix_fmt rgb48le \"" + (output_dir / "frame_%06d.tiff").string() +
                    "\" 2>/dev/null";
  int rc = system(cmd.c_str());
  if (rc != 0)
  {
    spdlog::error("Subtitle burn-in failed");
    return 1;
  }

  spdlog::info("Subtitle burn-in complete: {}", output_dir.string());
  return 0;
}

} // namespace dcpwizard
