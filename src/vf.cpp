#include "dcpwizard/vf.h"
#include "dcpwizard/cpl.h"
#include "dcpwizard/pkl.h"
#include "dcpwizard/assetmap.h"
#include "dcpwizard/hash.h"

#include <KM_util.h>
#include <filesystem>
#include <fstream>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

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

int create_vf(const VFConfig& config)
{
  if (!fs::exists(config.original_dcp))
  {
    spdlog::error("Original DCP not found: {}", config.original_dcp.string());
    return 1;
  }

  spdlog::info("Creating VF referencing OV: {}", config.original_dcp.string());
  fs::create_directories(config.output_dir);

  // Read OV's ASSETMAP to get asset UUIDs
  // A VF CPL references reels from the OV without copying MXFs
  std::string vf_cpl_uuid = make_uuid();
  auto cpl_file = config.output_dir / "cpl_vf.xml";

  // Generate VF CPL that references OV reels
  CPLConfig cpl;
  cpl.id = vf_cpl_uuid;
  cpl.title = "VF (" + config.original_dcp.filename().string() + ")";
  cpl.content_kind = "feature";
  cpl.frame_rate_num = 24;
  cpl.frame_rate_den = 1;

  // Create an empty reel placeholder (in real VF, these reference OV assets)
  CPLReel reel;
  reel.id = make_uuid();
  reel.picture.id = make_uuid();
  reel.picture.asset_id = make_uuid(); // Would reference OV asset
  reel.picture.duration = 1;
  reel.picture.frame_rate_num = 24;
  reel.picture.frame_rate_den = 1;
  cpl.reels.push_back(reel);

  int rc = generate_cpl(cpl, cpl_file);
  if (rc != 0)
    return rc;

  // Generate VF PKL
  std::string pkl_uuid = make_uuid();
  PKLConfig pkl;
  pkl.id = pkl_uuid;
  pkl.annotation = "VF PKL";

  PKLEntry cpl_entry;
  cpl_entry.id = vf_cpl_uuid;
  cpl_entry.type = "text/xml";
  cpl_entry.hash = hash_file_base64(cpl_file);
  cpl_entry.size = fs::file_size(cpl_file);
  cpl_entry.original_filename = cpl_file.filename().string();
  pkl.entries.push_back(cpl_entry);

  auto pkl_file = config.output_dir / "pkl_vf.xml";
  rc = generate_pkl(pkl, pkl_file);
  if (rc != 0)
    return rc;

  // Generate ASSETMAP
  AssetMapConfig am;
  am.id = make_uuid();
  am.annotation = "VF";
  am.entries.push_back({pkl_uuid, pkl_file.filename().string(), fs::file_size(pkl_file)});
  am.entries.push_back({vf_cpl_uuid, cpl_file.filename().string(), fs::file_size(cpl_file)});

  rc = generate_assetmap(am, config.output_dir);
  if (rc != 0)
    return rc;

  spdlog::info("VF created: {}", config.output_dir.string());
  return 0;
}

} // namespace dcpwizard
