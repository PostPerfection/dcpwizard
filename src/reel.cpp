#ifdef _WIN32
#define NOMINMAX
#endif

#include "dcpwizard/reel.h"

#include <KM_util.h>
#include <spdlog/spdlog.h>

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

std::vector<ReelInfo> plan_reels(uint64_t total_frames,
                                 uint32_t frame_rate,
                                 const ReelConfig& config)
{
  std::vector<ReelInfo> reels;

  if (config.split_mode == ReelSplitMode::None || total_frames == 0)
  {
    ReelInfo r;
    r.id = make_uuid();
    r.frame_start = 0;
    r.frame_end = total_frames;
    reels.push_back(r);
    return reels;
  }

  uint64_t frames_per_reel = total_frames; // default: no split

  if (config.split_mode == ReelSplitMode::ByDuration)
  {
    // Convert minutes to frames
    frames_per_reel = static_cast<uint64_t>(config.split_duration_minutes) * 60 * frame_rate;
  }
  else if (config.split_mode == ReelSplitMode::BySize && config.split_size_bytes > 0)
  {
    // Estimate frames per reel based on target size
    // Assume ~250 Mbps average bitrate
    double bytes_per_frame = (250.0 * 1000000.0) / (8.0 * frame_rate);
    frames_per_reel = static_cast<uint64_t>(config.split_size_bytes / bytes_per_frame);
  }

  if (frames_per_reel == 0)
    frames_per_reel = total_frames;

  uint64_t pos = 0;
  while (pos < total_frames)
  {
    ReelInfo r;
    r.id = make_uuid();
    r.frame_start = pos;
    r.frame_end = std::min(pos + frames_per_reel, total_frames);
    reels.push_back(r);
    pos = r.frame_end;
  }

  spdlog::info("Planned {} reels ({} frames/reel)", reels.size(), frames_per_reel);
  return reels;
}

} // namespace dcpwizard
