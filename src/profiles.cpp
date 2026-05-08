#include "dcpwizard/profiles.h"

namespace dcpwizard
{

Profile get_profile(const std::string& name)
{
  if (name == "cinema2k")
    return {"Cinema 2K", "SMPTE", "2K", 24, 250, false};
  if (name == "cinema4k")
    return {"Cinema 4K", "SMPTE", "4K", 24, 250, false};
  if (name == "cinema2k_encrypted")
    return {"Cinema 2K Encrypted", "SMPTE", "2K", 24, 250, true};
  if (name == "interop")
    return {"Interop", "Interop", "2K", 24, 250, false};
  if (name == "trailer")
    return {"Trailer", "SMPTE", "2K", 24, 250, false};
  // Default
  return {"Custom", "SMPTE", "2K", 24, 250, false};
}

} // namespace dcpwizard
