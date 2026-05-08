#include "dcpwizard/encrypt.h"
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int encrypt_dcp(const EncryptionConfig& config)
{
  spdlog::info("Encrypting DCP: {}", config.dcp_dir.string());
  // TODO: implement AES-128 encryption
  return 0;
}

} // namespace dcpwizard
