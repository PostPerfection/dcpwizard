#pragma once

#include <string>

namespace dcpwizard
{

enum class ColourSpace
{
  Rec709,
  P3_D65,
  P3_DCI,
  XYZ
};

enum class TransferFunction
{
  Gamma26,
  PQ,
  HLG,
  Linear
};

struct ColourConfig
{
  ColourSpace input_space = ColourSpace::Rec709;
  ColourSpace output_space = ColourSpace::XYZ;
  TransferFunction input_tf = TransferFunction::Gamma26;
  TransferFunction output_tf = TransferFunction::Gamma26;
};

/// Apply colour conversion to image data.
int convert_colour(const ColourConfig& config,
                   const void* input, void* output,
                   int width, int height);

} // namespace dcpwizard
