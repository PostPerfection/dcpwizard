#include "dcpwizard/dcp.h"
#include "dcpwizard/assetmap.h"
#include "dcpwizard/audio.h"
#include "dcpwizard/cpl.h"
#include "dcpwizard/encode.h"
#include "dcpwizard/hash.h"
#include "dcpwizard/mxf_wrap.h"
#include "dcpwizard/pkl.h"
#include "dcpwizard/transcode.h"

#include <KM_util.h>
#include <filesystem>
#include <spdlog/spdlog.h>

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

int create_dcp(const DCPConfig& config)
{
  spdlog::info("Creating DCP: '{}' ({})", config.title,
               config.standard == Standard::SMPTE ? "SMPTE" : "Interop");

  if (config.video_dir.empty() || !fs::exists(config.video_dir))
  {
    spdlog::error("Video input does not exist: {}", config.video_dir.string());
    return 1;
  }

  fs::create_directories(config.output_dir);

  // If input is a video file (MP4, MOV, etc.), transcode first
  fs::path frames_dir = config.video_dir;
  fs::path audio_file = config.audio_file;
  bool transcode_cleanup = false;

  if (is_video_file(config.video_dir))
  {
    spdlog::info("Input is a video file — transcoding to image sequence...");
    auto transcode_dir = config.output_dir / "transcode_tmp";

    TranscodeConfig tc;
    tc.input_file = config.video_dir;
    tc.output_dir = transcode_dir;

    auto result = transcode_to_sequence(tc);
    if (!result.success)
    {
      spdlog::error("Transcode failed: {}", result.error);
      return 1;
    }

    frames_dir = result.output_dir;
    transcode_cleanup = true;

    // Use extracted audio if no audio was explicitly specified
    if (audio_file.empty() && !result.audio_file.empty())
    {
      audio_file = result.audio_file;
      spdlog::info("Using extracted audio: {}", audio_file.string());
    }
  }
  else if (!fs::is_directory(config.video_dir))
  {
    spdlog::error("Video input is not a directory or video file: {}",
                  config.video_dir.string());
    return 1;
  }

  // Step 1: Encode images → J2K
  auto j2k_dir = config.output_dir / "j2k_tmp";
  fs::create_directories(j2k_dir);

  {
    EncodeConfig enc;
    enc.input_dir = frames_dir;
    enc.output_dir = j2k_dir;
    enc.bandwidth_mbps = config.max_bitrate_mbps;
    int rc = encode_j2k(enc);
    if (rc != 0)
    {
      spdlog::error("J2K encoding failed");
      return 1;
    }
  }

  // Count frames
  uint32_t frame_count = 0;
  for (const auto& entry : fs::directory_iterator(j2k_dir))
  {
    auto ext = entry.path().extension().string();
    if (ext == ".j2k" || ext == ".j2c")
      ++frame_count;
  }

  if (frame_count == 0)
  {
    spdlog::error("No J2K frames produced");
    return 1;
  }
  spdlog::info("Encoded {} frames", frame_count);

  // Step 2: Wrap J2K → picture MXF
  std::string picture_uuid = make_uuid();
  auto picture_mxf = config.output_dir / "picture.mxf";

  {
    MXFWrapConfig mxf;
    mxf.type = MXFType::Picture;
    mxf.input = j2k_dir;
    mxf.output = picture_mxf;
    mxf.frame_rate_num = config.frame_rate_num;
    mxf.frame_rate_den = config.frame_rate_den;
    int rc = wrap_mxf(mxf);
    if (rc != 0)
    {
      spdlog::error("Picture MXF wrapping failed");
      return 1;
    }
  }

  // Step 3: Wrap audio → sound MXF (optional)
  std::string sound_uuid;
  auto sound_mxf = config.output_dir / "audio.mxf";
  bool has_audio = !audio_file.empty() && fs::exists(audio_file);

  if (has_audio)
  {
    sound_uuid = make_uuid();
    AudioConfig acfg;
    acfg.input_files.push_back(audio_file);
    int rc = wrap_audio(acfg, sound_mxf);
    if (rc != 0)
    {
      spdlog::error("Audio MXF wrapping failed");
      return 1;
    }
  }

  // Step 4: Generate CPL
  std::string cpl_uuid = make_uuid();
  auto cpl_file = config.output_dir / "cpl.xml";

  {
    CPLConfig cpl;
    cpl.id = cpl_uuid;
    cpl.title = config.title;
    cpl.content_kind = "feature";
    cpl.frame_rate_num = config.frame_rate_num;
    cpl.frame_rate_den = config.frame_rate_den;

    CPLReel reel;
    reel.id = make_uuid();

    reel.picture.id = make_uuid();
    reel.picture.asset_id = picture_uuid;
    reel.picture.duration = frame_count;
    reel.picture.frame_rate_num = config.frame_rate_num;
    reel.picture.frame_rate_den = config.frame_rate_den;

    if (has_audio)
    {
      reel.sound.id = make_uuid();
      reel.sound.asset_id = sound_uuid;
      reel.sound.duration = frame_count;
      reel.sound.frame_rate_num = config.frame_rate_num;
      reel.sound.frame_rate_den = config.frame_rate_den;
    }

    cpl.reels.push_back(reel);

    int rc = generate_cpl(cpl, cpl_file);
    if (rc != 0)
      return 1;
  }

  // Step 5: Generate PKL
  std::string pkl_uuid = make_uuid();
  auto pkl_file = config.output_dir / "pkl.xml";

  {
    PKLConfig pkl;
    pkl.id = pkl_uuid;
    pkl.annotation = config.title;

    // Picture MXF entry
    PKLEntry pic_entry;
    pic_entry.id = picture_uuid;
    pic_entry.type = "application/mxf";
    pic_entry.hash = hash_file_base64(picture_mxf);
    pic_entry.size = fs::file_size(picture_mxf);
    pic_entry.original_filename = "picture.mxf";
    pkl.entries.push_back(pic_entry);

    // Sound MXF entry
    if (has_audio)
    {
      PKLEntry snd_entry;
      snd_entry.id = sound_uuid;
      snd_entry.type = "application/mxf";
      snd_entry.hash = hash_file_base64(sound_mxf);
      snd_entry.size = fs::file_size(sound_mxf);
      snd_entry.original_filename = "audio.mxf";
      pkl.entries.push_back(snd_entry);
    }

    // CPL entry
    PKLEntry cpl_entry;
    cpl_entry.id = cpl_uuid;
    cpl_entry.type = "text/xml";
    cpl_entry.hash = hash_file_base64(cpl_file);
    cpl_entry.size = fs::file_size(cpl_file);
    cpl_entry.original_filename = "cpl.xml";
    pkl.entries.push_back(cpl_entry);

    int rc = generate_pkl(pkl, pkl_file);
    if (rc != 0)
      return 1;
  }

  // Step 6: Generate ASSETMAP + VOLINDEX
  {
    AssetMapConfig am;
    am.id = make_uuid();
    am.annotation = config.title;

    am.entries.push_back({pkl_uuid, "pkl.xml", fs::file_size(pkl_file)});
    am.entries.push_back({cpl_uuid, "cpl.xml", fs::file_size(cpl_file)});
    am.entries.push_back({picture_uuid, "picture.mxf", fs::file_size(picture_mxf)});
    if (has_audio)
      am.entries.push_back({sound_uuid, "audio.mxf", fs::file_size(sound_mxf)});

    int rc = generate_assetmap(am, config.output_dir);
    if (rc != 0)
      return 1;
  }

  // Cleanup temp J2K directory
  fs::remove_all(j2k_dir);

  // Cleanup transcode temp directory
  if (transcode_cleanup)
    fs::remove_all(config.output_dir / "transcode_tmp");

  spdlog::info("DCP created successfully: {}", config.output_dir.string());
  return 0;
}

} // namespace dcpwizard
