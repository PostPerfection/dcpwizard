#include "dcpwizard/stereo3d.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int create_stereo3d(const Stereo3DConfig& config)
{
  spdlog::info("Creating stereoscopic 3D MXF");
  // TODO: implement stereoscopic frame interleaving
  return 0;
}

} // namespace dcpwizard
