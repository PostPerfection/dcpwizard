#include "dcpwizard/loudness.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

LoudnessResult measure_loudness(const std::filesystem::path& audio_file)
{
  spdlog::info("Measuring loudness: {}", audio_file.string());
  LoudnessResult result;
  // TODO: implement EBU R128 loudness measurement
  return result;
}

} // namespace dcpwizard
