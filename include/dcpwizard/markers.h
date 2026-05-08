#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace dcpwizard
{

enum class Marker
{
  FFOC,  // First Frame of Composition
  LFOC,  // Last Frame of Composition
  FFTC,  // First Frame of Title Credits
  LFTC,  // Last Frame of Title Credits
  FFOI,  // First Frame of Intermission
  LFOI,  // Last Frame of Intermission
  FFEC,  // First Frame of End Credits
  LFEC,  // Last Frame of End Credits
  FFMC,  // First Frame of Moving Credits
  LFMC   // Last Frame of Moving Credits
};

struct MarkerEntry
{
  Marker marker;
  uint64_t frame;
};

} // namespace dcpwizard
