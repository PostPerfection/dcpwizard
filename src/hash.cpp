#include "dcpwizard/hash.h"

#include <fstream>
#include <openssl/sha.h>
#include <spdlog/spdlog.h>
#include <sstream>
#include <iomanip>

namespace dcpwizard
{

std::string hash_file(const std::filesystem::path& file)
{
  std::ifstream in(file, std::ios::binary);
  if (!in)
  {
    spdlog::error("Cannot open file for hashing: {}", file.string());
    return {};
  }

  SHA_CTX ctx;
  SHA1_Init(&ctx);

  char buf[65536];
  while (in.read(buf, sizeof(buf)))
    SHA1_Update(&ctx, buf, in.gcount());
  if (in.gcount() > 0)
    SHA1_Update(&ctx, buf, in.gcount());

  unsigned char digest[SHA_DIGEST_LENGTH];
  SHA1_Final(digest, &ctx);

  std::ostringstream hex;
  for (int i = 0; i < SHA_DIGEST_LENGTH; ++i)
    hex << std::hex << std::setfill('0') << std::setw(2) << (int)digest[i];

  return hex.str();
}

} // namespace dcpwizard
