#include "dcpwizard/hfr.h"

namespace dcpwizard
{

bool is_valid_frame_rate(FrameRate fps, bool smpte)
{
  if (smpte)
  {
    switch (fps)
    {
    case FrameRate::FPS_24:
    case FrameRate::FPS_25:
    case FrameRate::FPS_30:
    case FrameRate::FPS_48:
    case FrameRate::FPS_60:
    case FrameRate::FPS_96:
    case FrameRate::FPS_120:
      return true;
    }
  }
  else
  {
    // Interop only supports 24/25
    switch (fps)
    {
    case FrameRate::FPS_24:
    case FrameRate::FPS_25:
      return true;
    default:
      return false;
    }
  }
  return false;
}

std::vector<FrameRate> supported_frame_rates(bool smpte)
{
  if (smpte)
    return {FrameRate::FPS_24, FrameRate::FPS_25, FrameRate::FPS_30,
            FrameRate::FPS_48, FrameRate::FPS_60, FrameRate::FPS_96,
            FrameRate::FPS_120};
  return {FrameRate::FPS_24, FrameRate::FPS_25};
}

} // namespace dcpwizard
