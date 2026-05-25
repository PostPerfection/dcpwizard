#include "dcpwizard/hash.h"

#include <fstream>
#include <openssl/bio.h>
#include <openssl/buffer.h>
#include <openssl/evp.h>
#include <openssl/sha.h>
#include <spdlog/spdlog.h>
#include <sstream>
#include <iomanip>

namespace dcpwizard
{

static std::string base64_encode(const unsigned char* data, size_t len)
{
  BIO* b64 = BIO_new(BIO_f_base64());
  BIO* mem = BIO_new(BIO_s_mem());
  b64 = BIO_push(b64, mem);
  BIO_set_flags(b64, BIO_FLAGS_BASE64_NO_NL);
  BIO_write(b64, data, static_cast<int>(len));
  BIO_flush(b64);

  BUF_MEM* bptr = nullptr;
  BIO_get_mem_ptr(b64, &bptr);
  std::string result(bptr->data, bptr->length);
  BIO_free_all(b64);
  return result;
}

static bool compute_sha1(const std::filesystem::path& file, unsigned char* digest)
{
  std::ifstream in(file, std::ios::binary);
  if (!in)
  {
    spdlog::error("Cannot open file for hashing: {}", file.string());
    return false;
  }

  SHA_CTX ctx;
  SHA1_Init(&ctx);

  char buf[65536];
  while (in.read(buf, sizeof(buf)))
    SHA1_Update(&ctx, buf, in.gcount());
  if (in.gcount() > 0)
    SHA1_Update(&ctx, buf, in.gcount());

  SHA1_Final(digest, &ctx);
  return true;
}

std::string hash_file_base64(const std::filesystem::path& file)
{
  unsigned char digest[SHA_DIGEST_LENGTH];
  if (!compute_sha1(file, digest))
    return {};
  return base64_encode(digest, SHA_DIGEST_LENGTH);
}

std::string hash_file(const std::filesystem::path& file)
{
  unsigned char digest[SHA_DIGEST_LENGTH];
  if (!compute_sha1(file, digest))
    return {};

  std::ostringstream hex;
  for (int i = 0; i < SHA_DIGEST_LENGTH; ++i)
    hex << std::hex << std::setfill('0') << std::setw(2) << (int)digest[i];

  return hex.str();
}

} // namespace dcpwizard
