#include "dcpwizard/mxf_wrap.h"

#include <AS_DCP.h>
#include <KM_util.h>
#include <spdlog/spdlog.h>

namespace dcpwizard
{

static void fill_writer_info(ASDCP::WriterInfo& info, bool smpte)
{
  info.LabelSetType = smpte ? ASDCP::LS_MXF_SMPTE : ASDCP::LS_MXF_INTEROP;
  Kumu::GenRandomUUID(info.AssetUUID);
  Kumu::GenRandomUUID(info.ContextID);
  info.CompanyName = "dcpwizard";
  info.ProductName = "DCP Wizard";
  info.ProductVersion = "0.1.0";
  info.EncryptedEssence = false;
  info.UsesHMAC = false;
}

int wrap_mxf(const MXFWrapConfig& config)
{
  if (config.type == MXFType::Picture)
  {
    // Wrap J2K codestream directory → picture MXF
    ASDCP::JP2K::SequenceParser parser;
    ASDCP::Result_t result = parser.OpenRead(config.input.string());
    if (ASDCP_FAILURE(result))
    {
      spdlog::error("Failed to open J2K sequence at '{}': {}",
                    config.input.string(), result.Label());
      return 1;
    }

    ASDCP::JP2K::PictureDescriptor pdesc;
    result = parser.FillPictureDescriptor(pdesc);
    if (ASDCP_FAILURE(result))
    {
      spdlog::error("Failed to parse J2K picture descriptor: {}", result.Label());
      return 1;
    }

    pdesc.EditRate = ASDCP::Rational(config.frame_rate_num, config.frame_rate_den);

    ASDCP::WriterInfo winfo;
    fill_writer_info(winfo, true);

    ASDCP::JP2K::MXFWriter writer;
    result = writer.OpenWrite(config.output.string(), winfo, pdesc);
    if (ASDCP_FAILURE(result))
    {
      spdlog::error("Failed to open MXF for writing '{}': {}",
                    config.output.string(), result.Label());
      return 1;
    }

    ASDCP::JP2K::FrameBuffer frame_buf(4 * 1024 * 1024); // 4MB buffer
    uint32_t frame_count = 0;

    while (ASDCP_SUCCESS(parser.ReadFrame(frame_buf)))
    {
      result = writer.WriteFrame(frame_buf);
      if (ASDCP_FAILURE(result))
      {
        spdlog::error("Failed to write frame {}: {}", frame_count, result.Label());
        return 1;
      }
      ++frame_count;
    }

    result = writer.Finalize();
    if (ASDCP_FAILURE(result))
    {
      spdlog::error("Failed to finalize MXF: {}", result.Label());
      return 1;
    }

    spdlog::info("Wrapped {} J2K frames → {}", frame_count, config.output.string());
    return 0;
  }

  spdlog::error("Unsupported MXF type");
  return 1;
}

} // namespace dcpwizard
