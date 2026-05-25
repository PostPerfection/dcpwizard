#include "dcpwizard/audio.h"

#include <AS_DCP.h>
#include <KM_util.h>
#include <spdlog/spdlog.h>

namespace dcpwizard
{

int wrap_audio(const AudioConfig& config, const std::filesystem::path& output_mxf)
{
  if (config.input_files.empty())
  {
    spdlog::error("No audio input files specified");
    return 1;
  }

  // DCI audio: 24-bit, 48kHz, frame-wrapped
  ASDCP::Rational edit_rate(24, 1); // match picture rate

  ASDCP::PCM::WAVParser parser;
  ASDCP::Result_t result = parser.OpenRead(config.input_files[0].string(), edit_rate);
  if (ASDCP_FAILURE(result))
  {
    spdlog::error("Failed to open WAV file '{}': {}",
                  config.input_files[0].string(), result.Label());
    return 1;
  }

  ASDCP::PCM::AudioDescriptor adesc;
  result = parser.FillAudioDescriptor(adesc);
  if (ASDCP_FAILURE(result))
  {
    spdlog::error("Failed to read audio descriptor: {}", result.Label());
    return 1;
  }

  adesc.EditRate = edit_rate;

  ASDCP::WriterInfo winfo;
  winfo.LabelSetType = ASDCP::LS_MXF_SMPTE;
  Kumu::GenRandomUUID(winfo.AssetUUID);
  Kumu::GenRandomUUID(winfo.ContextID);
  winfo.CompanyName = "dcpwizard";
  winfo.ProductName = "DCP Wizard";
  winfo.ProductVersion = "0.1.0";

  ASDCP::PCM::MXFWriter writer;
  result = writer.OpenWrite(output_mxf.string(), winfo, adesc);
  if (ASDCP_FAILURE(result))
  {
    spdlog::error("Failed to open audio MXF for writing '{}': {}",
                  output_mxf.string(), result.Label());
    return 1;
  }

  ASDCP::PCM::FrameBuffer frame_buf(ASDCP::PCM::CalcFrameBufferSize(adesc));
  uint32_t frame_count = 0;

  while (ASDCP_SUCCESS(parser.ReadFrame(frame_buf)))
  {
    result = writer.WriteFrame(frame_buf);
    if (ASDCP_FAILURE(result))
    {
      spdlog::error("Failed to write audio frame {}: {}", frame_count, result.Label());
      return 1;
    }
    ++frame_count;
  }

  result = writer.Finalize();
  if (ASDCP_FAILURE(result))
  {
    spdlog::error("Failed to finalize audio MXF: {}", result.Label());
    return 1;
  }

  spdlog::info("Wrapped {} audio frames → {}", frame_count, output_mxf.string());
  return 0;
}

} // namespace dcpwizard
