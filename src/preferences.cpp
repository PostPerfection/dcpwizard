#include "dcpwizard/preferences.h"

#include <spdlog/spdlog.h>

#include <cstdlib>
#include <fstream>
#include <sstream>

// Minimal JSON read/write without external dependency.
// Uses a simple key-value approach for flat + array fields.

namespace dcpwizard
{

std::filesystem::path preferences_path()
{
#ifdef _WIN32
  const char* appdata = std::getenv("APPDATA");
  if (appdata)
    return std::filesystem::path(appdata) / "dcpwizard" / "preferences.json";
  return "preferences.json";
#elif defined(__APPLE__)
  const char* home = std::getenv("HOME");
  if (home)
    return std::filesystem::path(home) / "Library" / "Application Support" / "dcpwizard" / "preferences.json";
  return "preferences.json";
#else
  const char* xdg = std::getenv("XDG_CONFIG_HOME");
  if (xdg)
    return std::filesystem::path(xdg) / "dcpwizard" / "preferences.json";
  const char* home = std::getenv("HOME");
  if (home)
    return std::filesystem::path(home) / ".config" / "dcpwizard" / "preferences.json";
  return "preferences.json";
#endif
}

// Simple JSON string extraction helper
static std::string json_string(const std::string& json, const std::string& key)
{
  auto pos = json.find("\"" + key + "\"");
  if (pos == std::string::npos) return "";
  pos = json.find(':', pos);
  if (pos == std::string::npos) return "";
  pos = json.find('"', pos + 1);
  if (pos == std::string::npos) return "";
  auto end = json.find('"', pos + 1);
  if (end == std::string::npos) return "";
  return json.substr(pos + 1, end - pos - 1);
}

static int json_int(const std::string& json, const std::string& key, int def)
{
  auto pos = json.find("\"" + key + "\"");
  if (pos == std::string::npos) return def;
  pos = json.find(':', pos);
  if (pos == std::string::npos) return def;
  pos++;
  while (pos < json.size() && (json[pos] == ' ' || json[pos] == '\t')) pos++;
  try { return std::stoi(json.substr(pos)); } catch (...) { return def; }
}

static double json_double(const std::string& json, const std::string& key, double def)
{
  auto pos = json.find("\"" + key + "\"");
  if (pos == std::string::npos) return def;
  pos = json.find(':', pos);
  if (pos == std::string::npos) return def;
  pos++;
  while (pos < json.size() && (json[pos] == ' ' || json[pos] == '\t')) pos++;
  try { return std::stod(json.substr(pos)); } catch (...) { return def; }
}

static bool json_bool(const std::string& json, const std::string& key, bool def)
{
  auto pos = json.find("\"" + key + "\"");
  if (pos == std::string::npos) return def;
  pos = json.find(':', pos);
  if (pos == std::string::npos) return def;
  pos++;
  while (pos < json.size() && (json[pos] == ' ' || json[pos] == '\t')) pos++;
  if (json.substr(pos, 4) == "true") return true;
  if (json.substr(pos, 5) == "false") return false;
  return def;
}

Preferences load_preferences()
{
  Preferences prefs;
  auto path = preferences_path();

  if (!std::filesystem::exists(path))
  {
    spdlog::debug("No preferences file at {}, using defaults", path.string());
    return prefs;
  }

  std::ifstream f(path);
  if (!f.is_open()) return prefs;

  std::ostringstream ss;
  ss << f.rdbuf();
  std::string json = ss.str();

  auto s = [&](const std::string& key, std::string& field) {
    auto v = json_string(json, key);
    if (!v.empty()) field = v;
  };

  s("default_standard", prefs.default_standard);
  s("default_resolution", prefs.default_resolution);
  prefs.default_frame_rate = static_cast<uint32_t>(json_int(json, "default_frame_rate", 24));
  s("creator_name", prefs.creator_name);
  s("isdcf_facility_code", prefs.isdcf_facility_code);
  s("preferred_encoder", prefs.preferred_encoder);
  prefs.default_bandwidth_mbps = static_cast<uint32_t>(json_int(json, "default_bandwidth_mbps", 250));
  s("default_colour_space", prefs.default_colour_space);
  prefs.gpu_device = json_int(json, "gpu_device", -1);
  s("signing_certificate_path", prefs.signing_certificate_path);
  s("signing_key_path", prefs.signing_key_path);
  s("intermediate_ca_path", prefs.intermediate_ca_path);
  s("kdm_annotation_pattern", prefs.kdm_annotation_pattern);
  prefs.kdm_validity_hours = json_int(json, "kdm_validity_hours", 168);
  s("default_output_dir", prefs.default_output_dir);
  s("naming_template", prefs.naming_template);
  s("default_channel_config", prefs.default_channel_config);
  prefs.loudness_target_lufs = json_double(json, "loudness_target_lufs", -24.0);
  s("theme", prefs.theme);
  prefs.show_advanced_options = json_bool(json, "show_advanced_options", false);

  spdlog::debug("Loaded preferences from {}", path.string());
  return prefs;
}

static std::string escape_json(const std::string& s)
{
  std::string out;
  for (char c : s)
  {
    if (c == '"') out += "\\\"";
    else if (c == '\\') out += "\\\\";
    else out += c;
  }
  return out;
}

int save_preferences(const Preferences& prefs)
{
  auto path = preferences_path();
  std::filesystem::create_directories(path.parent_path());

  std::ofstream f(path);
  if (!f.is_open())
  {
    spdlog::error("Failed to write preferences to {}", path.string());
    return 1;
  }

  f << "{\n";
  f << "  \"default_standard\": \"" << escape_json(prefs.default_standard) << "\",\n";
  f << "  \"default_resolution\": \"" << escape_json(prefs.default_resolution) << "\",\n";
  f << "  \"default_frame_rate\": " << prefs.default_frame_rate << ",\n";
  f << "  \"creator_name\": \"" << escape_json(prefs.creator_name) << "\",\n";
  f << "  \"isdcf_facility_code\": \"" << escape_json(prefs.isdcf_facility_code) << "\",\n";
  f << "  \"preferred_encoder\": \"" << escape_json(prefs.preferred_encoder) << "\",\n";
  f << "  \"default_bandwidth_mbps\": " << prefs.default_bandwidth_mbps << ",\n";
  f << "  \"default_colour_space\": \"" << escape_json(prefs.default_colour_space) << "\",\n";
  f << "  \"gpu_device\": " << prefs.gpu_device << ",\n";
  f << "  \"signing_certificate_path\": \"" << escape_json(prefs.signing_certificate_path) << "\",\n";
  f << "  \"signing_key_path\": \"" << escape_json(prefs.signing_key_path) << "\",\n";
  f << "  \"intermediate_ca_path\": \"" << escape_json(prefs.intermediate_ca_path) << "\",\n";
  f << "  \"kdm_annotation_pattern\": \"" << escape_json(prefs.kdm_annotation_pattern) << "\",\n";
  f << "  \"kdm_validity_hours\": " << prefs.kdm_validity_hours << ",\n";
  f << "  \"default_output_dir\": \"" << escape_json(prefs.default_output_dir) << "\",\n";
  f << "  \"naming_template\": \"" << escape_json(prefs.naming_template) << "\",\n";
  f << "  \"default_channel_config\": \"" << escape_json(prefs.default_channel_config) << "\",\n";
  f << "  \"loudness_target_lufs\": " << prefs.loudness_target_lufs << ",\n";
  f << "  \"theme\": \"" << escape_json(prefs.theme) << "\",\n";
  f << "  \"show_advanced_options\": " << (prefs.show_advanced_options ? "true" : "false") << ",\n";
  f << "  \"theater_list\": [\n";
  for (size_t i = 0; i < prefs.theater_list.size(); i++)
  {
    f << "    {\"name\": \"" << escape_json(prefs.theater_list[i].name)
      << "\", \"certificate_path\": \"" << escape_json(prefs.theater_list[i].certificate_path) << "\"}";
    if (i + 1 < prefs.theater_list.size()) f << ",";
    f << "\n";
  }
  f << "  ]\n";
  f << "}\n";

  spdlog::info("Saved preferences to {}", path.string());
  return 0;
}

} // namespace dcpwizard
