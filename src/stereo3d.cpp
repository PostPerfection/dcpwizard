#include "dcpwizard/stereo3d.h"
#include "dcpwizard/mxf_wrap.h"

#include <algorithm>
#include <filesystem>
#include <spdlog/spdlog.h>
#include <vector>

namespace fs = std::filesystem;

namespace dcpwizard
{

int create_stereo3d(const Stereo3DConfig& config)
{
  if (!fs::exists(config.left_dir) || !fs::exists(config.right_dir))
  {
    spdlog::error("Left or right eye directory not found");
    return 1;
  }

  spdlog::info("Creating stereoscopic 3D MXF: L={} R={}",
               config.left_dir.string(), config.right_dir.string());

  // Collect and sort left/right frames
  std::vector<fs::path> left_frames, right_frames;
  for (const auto& entry : fs::directory_iterator(config.left_dir))
    if (entry.path().extension() == ".j2k" || entry.path().extension() == ".j2c")
      left_frames.push_back(entry.path());

  for (const auto& entry : fs::directory_iterator(config.right_dir))
    if (entry.path().extension() == ".j2k" || entry.path().extension() == ".j2c")
      right_frames.push_back(entry.path());

  std::sort(left_frames.begin(), left_frames.end());
  std::sort(right_frames.begin(), right_frames.end());

  if (left_frames.size() != right_frames.size())
  {
    spdlog::error("Frame count mismatch: L={} R={}",
                  left_frames.size(), right_frames.size());
    return 1;
  }

  if (left_frames.empty())
  {
    spdlog::error("No J2K frames found");
    return 1;
  }

  // Create interleaved directory (L,R,L,R,...)
  auto interleaved_dir = config.output_mxf.parent_path() / "stereo_interleaved";
  fs::create_directories(interleaved_dir);

  uint32_t idx = 0;
  for (size_t i = 0; i < left_frames.size(); ++i)
  {
    char name[64];
    snprintf(name, sizeof(name), "frame_%06u.j2c", idx++);
    fs::copy_file(left_frames[i], interleaved_dir / name, fs::copy_options::overwrite_existing);
    snprintf(name, sizeof(name), "frame_%06u.j2c", idx++);
    fs::copy_file(right_frames[i], interleaved_dir / name, fs::copy_options::overwrite_existing);
  }

  // Wrap interleaved frames as MXF (double frame rate for stereo)
  MXFWrapConfig mxf;
  mxf.type = MXFType::Picture;
  mxf.input = interleaved_dir;
  mxf.output = config.output_mxf;
  mxf.frame_rate_num = config.frame_rate_num * 2;
  mxf.frame_rate_den = config.frame_rate_den;

  int rc = wrap_mxf(mxf);

  // Cleanup interleaved dir
  fs::remove_all(interleaved_dir);

  if (rc == 0)
    spdlog::info("Stereoscopic MXF created: {} ({} frame pairs)",
                 config.output_mxf.string(), left_frames.size());

  return rc;
}

} // namespace dcpwizard
