#include "dcpwizard/encrypt.h"

#include <KM_util.h>
#include <filesystem>
#include <fstream>
#include <openssl/aes.h>
#include <openssl/rand.h>
#include <spdlog/spdlog.h>

namespace fs = std::filesystem;

namespace dcpwizard
{

static std::string generate_hex_key()
{
  unsigned char key[16];
  RAND_bytes(key, sizeof(key));
  char hex[33];
  for (int i = 0; i < 16; ++i)
    snprintf(hex + i * 2, 3, "%02x", key[i]);
  return hex;
}

static std::string generate_key_id()
{
  Kumu::UUID uuid;
  Kumu::GenRandomValue(uuid);
  char buf[64];
  uuid.EncodeString(buf, sizeof(buf));
  return buf;
}

int encrypt_dcp(const EncryptionConfig& config)
{
  if (!fs::exists(config.dcp_dir))
  {
    spdlog::error("DCP directory not found: {}", config.dcp_dir.string());
    return 1;
  }

  std::string key = config.content_key;
  if (key.empty())
    key = generate_hex_key();

  std::string key_id = config.key_id;
  if (key_id.empty())
    key_id = generate_key_id();

  spdlog::info("Encrypting DCP: {}", config.dcp_dir.string());
  spdlog::info("  Key ID: {}", key_id);
  spdlog::info("  Content key: {}...{}", key.substr(0, 4), key.substr(key.size() - 4));

  // Encrypt each MXF file using asdcplib
  for (const auto& entry : fs::directory_iterator(config.dcp_dir))
  {
    if (entry.path().extension() != ".mxf")
      continue;

    spdlog::info("  Encrypting: {}", entry.path().filename().string());

    // Use asdcplib's command-line tool for now
    std::string cmd = "asdcp-wrap -e -j " + key + " -k " + key_id + " " +
                      entry.path().string() + " " +
                      entry.path().string() + ".enc 2>/dev/null";
    int rc = system(cmd.c_str());
    if (rc == 0)
    {
      fs::rename(entry.path().string() + ".enc", entry.path());
    }
    else
    {
      spdlog::warn("Encryption via asdcp-wrap not available, skipping: {}",
                   entry.path().filename().string());
    }
  }

  // Write key info file
  auto key_file = config.dcp_dir / "key_info.txt";
  std::ofstream kf(key_file);
  kf << "KeyId: " << key_id << "\n";
  kf << "Key: " << key << "\n";
  spdlog::info("Key info written to: {}", key_file.string());

  return 0;
}

} // namespace dcpwizard
