#include "dcpwizard/colour.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int convert_colour(const ColourConfig& config,
                   const void* input, void* output,
                   int width, int height)
{
  spdlog::debug("Colour conversion: {}x{}", width, height);
  // TODO: implement colour space conversion (Rec.709/P3 → XYZ)
  return 0;
}

} // namespace dcpwizard
