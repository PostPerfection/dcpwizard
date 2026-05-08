#include "dcpwizard/reel.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

std::vector<ReelInfo> plan_reels(uint64_t total_frames,
                                 uint32_t frame_rate,
                                 const ReelConfig& config)
{
  std::vector<ReelInfo> reels;
  if (config.split_mode == ReelSplitMode::None)
  {
    ReelInfo r;
    r.frame_start = 0;
    r.frame_end = total_frames;
    reels.push_back(r);
    return reels;
  }
  // TODO: implement reel splitting logic
  return reels;
}

} // namespace dcpwizard
