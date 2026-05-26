#include "dcpwizard/j2k_transcode.h"
#include "dcpwizard/encode.h"

#include <algorithm>
#include <filesystem>
#include <spdlog/spdlog.h>
#include <vector>

namespace fs = std::filesystem;

namespace dcpwizard
{

int transcode_j2k(const J2KTranscodeConfig& config)
{
  if (!fs::exists(config.input_dir))
  {
    spdlog::error("Input J2K directory not found: {}", config.input_dir.string());
    return 1;
  }

  spdlog::info("Transcoding J2K: {} → {} @ {} Mbps",
               config.input_dir.string(), config.output_dir.string(),
               config.target_bandwidth_mbps);

  fs::create_directories(config.output_dir);

  if (config.decode_first)
  {
    // Decode J2K to TIFF, then re-encode at target bitrate
    auto tmp_decoded = config.output_dir.parent_path() / "j2k_decode_tmp";
    fs::create_directories(tmp_decoded);

    // Decode: grk_decompress batch
    std::string decode_cmd = "grk_decompress --batch-src " + config.input_dir.string() +
                             " -a " + tmp_decoded.string() + " -O tif 2>/dev/null";
    int rc = system(decode_cmd.c_str());
    if (rc != 0)
    {
      spdlog::error("J2K decode failed");
      fs::remove_all(tmp_decoded);
      return 1;
    }

    // Re-encode at target bitrate
    EncodeConfig enc;
    enc.input_dir = tmp_decoded;
    enc.output_dir = config.output_dir;
    enc.bandwidth_mbps = config.target_bandwidth_mbps;
    rc = encode_j2k(enc);

    fs::remove_all(tmp_decoded);
    return rc;
  }
  else
  {
    // Direct re-encode (copy with different parameters)
    EncodeConfig enc;
    enc.input_dir = config.input_dir;
    enc.output_dir = config.output_dir;
    enc.bandwidth_mbps = config.target_bandwidth_mbps;
    return encode_j2k(enc);
  }
}

} // namespace dcpwizard
