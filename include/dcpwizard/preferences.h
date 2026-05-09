#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

/// A cinema/theater entry for batch KDM generation.
struct TheaterEntry
{
  std::string name;
  std::string certificate_path;
};

/// User preferences stored in ~/.config/dcpwizard/preferences.json
/// (XDG_CONFIG_HOME on Linux, %APPDATA% on Windows, ~/Library/Application Support on macOS)
struct Preferences
{
  // General
  std::string default_standard = "SMPTE";       // "SMPTE" or "Interop"
  std::string default_resolution = "2K";         // "2K" or "4K"
  uint32_t default_frame_rate = 24;
  std::string creator_name;                      // e.g. "My Studio"
  std::string isdcf_facility_code;               // e.g. "MST"

  // Encoding
  std::string preferred_encoder = "grok";
  uint32_t default_bandwidth_mbps = 250;
  std::string default_colour_space = "Rec.709";  // "Rec.709", "P3-D65", "P3-DCI"
  int gpu_device = -1;                           // -1 = auto

  // Encryption & KDM
  std::string signing_certificate_path;
  std::string signing_key_path;
  std::string intermediate_ca_path;
  std::string kdm_annotation_pattern = "%t_%d";  // %t=title, %d=date
  int kdm_validity_hours = 168;                  // 7 days default
  std::vector<TheaterEntry> theater_list;

  // Delivery
  std::string default_output_dir;
  std::string naming_template;                   // ISDCF naming pattern

  // Audio
  std::string default_channel_config = "5.1";    // "mono","stereo","5.1","7.1"
  double loudness_target_lufs = -24.0;           // EBU R128

  // GUI
  std::string theme = "dark";
  bool show_advanced_options = false;
};

/// Get the platform-specific preferences file path.
std::filesystem::path preferences_path();

/// Load preferences from disk (returns defaults if file doesn't exist).
Preferences load_preferences();

/// Save preferences to disk.
int save_preferences(const Preferences& prefs);

} // namespace dcpwizard
