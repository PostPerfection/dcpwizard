#include "dcpwizard/colour.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <spdlog/spdlog.h>

namespace dcpwizard
{

// 3x3 matrix multiply for RGB→XYZ style transforms
static void mat3_multiply(const double m[9], const double in[3], double out[3])
{
  out[0] = m[0] * in[0] + m[1] * in[1] + m[2] * in[2];
  out[1] = m[3] * in[0] + m[4] * in[1] + m[5] * in[2];
  out[2] = m[6] * in[0] + m[7] * in[1] + m[8] * in[2];
}

// Rec.709 → XYZ (D65 adapted for DCI)
static constexpr double kRec709ToXYZ[9] = {
    0.4124564, 0.3575761, 0.1804375,
    0.2126729, 0.7151522, 0.0721750,
    0.0193339, 0.1191920, 0.9503041};

// P3-D65 → XYZ
static constexpr double kP3D65ToXYZ[9] = {
    0.4865709, 0.2656677, 0.1982173,
    0.2289746, 0.6917385, 0.0792869,
    0.0000000, 0.0451134, 1.0439444};

// P3-DCI → XYZ (DCI white point)
static constexpr double kP3DCIToXYZ[9] = {
    0.4451698, 0.2771344, 0.1722827,
    0.2094917, 0.7215953, 0.0689131,
    0.0000000, 0.0470606, 0.9073554};

static const double* get_matrix(ColourSpace from)
{
  switch (from)
  {
    case ColourSpace::Rec709:
      return kRec709ToXYZ;
    case ColourSpace::P3_D65:
      return kP3D65ToXYZ;
    case ColourSpace::P3_DCI:
      return kP3DCIToXYZ;
    default:
      return nullptr; // XYZ→XYZ = identity
  }
}

// Gamma 2.6 decode (DCI standard)
static double gamma_decode(double v, TransferFunction tf)
{
  switch (tf)
  {
    case TransferFunction::Gamma26:
      return std::pow(std::max(v, 0.0), 2.6);
    case TransferFunction::Linear:
      return v;
    default:
      return std::pow(std::max(v, 0.0), 2.6);
  }
}

// Gamma 2.6 encode
static double gamma_encode(double v, TransferFunction tf)
{
  switch (tf)
  {
    case TransferFunction::Gamma26:
      return std::pow(std::max(v, 0.0), 1.0 / 2.6);
    case TransferFunction::Linear:
      return v;
    default:
      return std::pow(std::max(v, 0.0), 1.0 / 2.6);
  }
}

int convert_colour(const ColourConfig& config,
                   const void* input, void* output,
                   int width, int height)
{
  if (!input || !output || width <= 0 || height <= 0)
    return 1;

  // Identity transform
  if (config.input_space == config.output_space &&
      config.input_tf == config.output_tf)
  {
    std::memcpy(output, input, static_cast<size_t>(width) * height * 6);
    return 0;
  }

  const double* matrix = get_matrix(config.input_space);
  if (!matrix && config.input_space != ColourSpace::XYZ)
  {
    spdlog::error("Unsupported colour space conversion");
    return 1;
  }

  // Process 16-bit RGB pixels
  const uint16_t* src = static_cast<const uint16_t*>(input);
  uint16_t* dst = static_cast<uint16_t*>(output);
  const double scale = 1.0 / 65535.0;

  for (int i = 0; i < width * height; ++i)
  {
    double r = src[i * 3 + 0] * scale;
    double g = src[i * 3 + 1] * scale;
    double b = src[i * 3 + 2] * scale;

    // Linearize
    r = gamma_decode(r, config.input_tf);
    g = gamma_decode(g, config.input_tf);
    b = gamma_decode(b, config.input_tf);

    double out[3];
    if (matrix)
    {
      double in[3] = {r, g, b};
      mat3_multiply(matrix, in, out);
    }
    else
    {
      out[0] = r;
      out[1] = g;
      out[2] = b;
    }

    // Apply output transfer function
    out[0] = gamma_encode(out[0], config.output_tf);
    out[1] = gamma_encode(out[1], config.output_tf);
    out[2] = gamma_encode(out[2], config.output_tf);

    // Clamp and quantize to 16-bit
    dst[i * 3 + 0] = static_cast<uint16_t>(std::clamp(out[0] * 65535.0, 0.0, 65535.0));
    dst[i * 3 + 1] = static_cast<uint16_t>(std::clamp(out[1] * 65535.0, 0.0, 65535.0));
    dst[i * 3 + 2] = static_cast<uint16_t>(std::clamp(out[2] * 65535.0, 0.0, 65535.0));
  }

  spdlog::debug("Colour conversion: {}x{} pixels processed", width, height);
  return 0;
}

} // namespace dcpwizard
