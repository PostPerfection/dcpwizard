#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace dcpwizard
{

struct EncryptionConfig
{
  std::filesystem::path dcp_dir;
  std::string content_key;       // hex string, empty = generate
  std::string key_id;            // UUID, empty = generate
};

/// Encrypt a DCP in-place with AES-128 content encryption.
int encrypt_dcp(const EncryptionConfig& config);

} // namespace dcpwizard
